use std::{
  collections::{BTreeMap, HashMap},
  time::Instant,
};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::{
  dds::{
    participant::DomainParticipant, qos::HasQoSPolicy, reader::ReaderIngredients,
    rtps_reader_proxy::RtpsReaderProxy, topic::Topic, traits::TopicDescription,
  },
  structure::{
    duration::Duration,
    entity::RTPSEntity,
    guid::{EntityId, GuidPrefix, GUID},
  },
};
use super::data_types::{
  spdp_participant_data::SpdpDiscoveredParticipantData,
  topic_data::{
    DiscoveredReaderData, DiscoveredTopicData, DiscoveredWriterData, ParticipantMessageData,
    ReaderProxy, SubscriptionBuiltinTopicData, TopicBuiltinTopicData,
  },
};

// If remote participant does not specifiy lease duration, how long silence
// until we pronounce it dead.
const DEFAULT_PARTICIPANT_LEASE_DURATION: Duration = Duration::from_secs(60);

// How much longer to wait than lease duration before pronouncing lost.
const PARTICIPANT_LEASE_DURATION_TOLREANCE: Duration = Duration::from_secs(0);

// TODO: Let DiscoveryDB itself become thread-safe and support smaller-scope
// lock
pub(crate) struct DiscoveryDB {
  my_guid: GUID,
  participant_proxies: BTreeMap<GuidPrefix, SpdpDiscoveredParticipantData>,
  participant_last_life_signs: HashMap<GuidPrefix, Instant>,
  // local writer proxies for topics (topic name acts as key)
  local_topic_writers: HashMap<GUID, DiscoveredWriterData>,
  // local reader proxies for topics (topic name acts as key)
  local_topic_readers: HashMap<GUID, DiscoveredReaderData>,

  // remote readers and writers (via discovery)
  external_topic_readers: BTreeMap<GUID, DiscoveredReaderData>,
  external_topic_writers: BTreeMap<GUID, DiscoveredWriterData>,

  topics: HashMap<String, DiscoveredTopicData>,

  event_sender: Option<mio_extras::channel::SyncSender<()>>,
  readers_updated: bool,
  writers_updated: bool,
}

impl DiscoveryDB {
  pub fn new(
    my_guid: GUID,
    event_sender: Option<mio_extras::channel::SyncSender<()>>,
  ) -> DiscoveryDB {
    DiscoveryDB {
      my_guid,
      participant_proxies: BTreeMap::new(),
      participant_last_life_signs: HashMap::new(),
      local_topic_writers: HashMap::new(),
      local_topic_readers: HashMap::new(),
      external_topic_readers: BTreeMap::new(),
      external_topic_writers: BTreeMap::new(),
      topics: HashMap::new(),
      event_sender,
      readers_updated: false,
      writers_updated: false,
    }
  }

  // Returns if particiapnt was previously unkonwn
  pub fn update_participant(&mut self, data: &SpdpDiscoveredParticipantData) -> bool {
    debug!("update_participant: {:?}", &data);
    let guid = data.participant_guid;

    // sanity check
    if guid.entity_id != EntityId::ENTITYID_PARTICIPANT {
      error!(
        "Discovered participant GUID entity_id is not for participant: {:?}",
        guid
      );
      // Maybe we should discard the participant here?
      return false;
    }

    // We allow discovery to discover self, since our discovery readers
    // will receive our own announcements via broadcast. If we do not recognize
    // our own participant, there is confusion about unknown writers on the
    // discovery topics.
    //
    // if guid == self.my_guid {
    //   debug!("DiscoveryDB discovered self. Skipping.");
    //   return
    // }

    let mut new_participant = false;
    if self.participant_proxies.get(&guid.guid_prefix).is_none() {
      info!("New remote participant: {:?}", &data);
      new_participant = true;
      if guid == self.my_guid {
        info!(
          "Remote participant {:?} is myself, but some reflection is good.",
          guid
        );
        new_participant = false;
      }
    }
    // actual work here:
    self
      .participant_proxies
      .insert(guid.guid_prefix, data.clone());
    self
      .participant_last_life_signs
      .insert(guid.guid_prefix, Instant::now());

    new_participant
  }

  pub fn remove_participant(&mut self, guid_prefix: GuidPrefix) {
    info!("removing participant {:?}", guid_prefix);
    self.participant_proxies.remove(&guid_prefix);
    self.participant_last_life_signs.remove(&guid_prefix);

    self.remove_topic_reader_with_prefix(guid_prefix);

    self.remove_topic_writer_with_prefix(guid_prefix);
  }

  pub fn find_participant_proxy(
    &self,
    guid_prefix: GuidPrefix,
  ) -> Option<&SpdpDiscoveredParticipantData> {
    self.participant_proxies.get(&guid_prefix)
  }

  pub fn find_remote_reader(&self, guid: GUID) -> Option<&DiscoveredReaderData> {
    self.external_topic_readers.get(&guid)
  }

  fn remove_topic_reader_with_prefix(&mut self, guid_prefix: GuidPrefix) {
    // TODO: Implement this using .drain_filter() in BTreeMap once it lands in
    // stable.
    let to_remove: Vec<GUID> = self
      .external_topic_readers
      .range(guid_prefix.range())
      .map(|(g, _)| *g)
      .collect();
    for guid in to_remove {
      self.external_topic_readers.remove(&guid);
    }
  }

  pub fn remove_topic_reader(&mut self, guid: GUID) {
    info!("remove_topic_reader {:?}", guid);
    self.external_topic_readers.remove(&guid);
  }

  fn remove_topic_writer_with_prefix(&mut self, guid_prefix: GuidPrefix) {
    // TODO: Implement this using .drain_filter() in BTreeMap once it lands in
    // stable.
    let to_remove: Vec<GUID> = self
      .external_topic_writers
      .range(guid_prefix.range())
      .map(|(g, _)| *g)
      .collect();
    for guid in to_remove {
      self.external_topic_writers.remove(&guid);
    }
  }

  pub fn remove_topic_writer(&mut self, guid: GUID) {
    self.external_topic_writers.remove(&guid);
  }

  // Delete participant proxies, if we have not heard of them within
  // lease_duration
  pub fn participant_cleanup(&mut self) -> Vec<GuidPrefix> {
    let inow = Instant::now();

    let mut to_remove = Vec::new();
    // TODO: We are not cleaning up liast_life_signs table, but that should not be a
    // problem, except for a slight memory leak.
    for (&guid, sp) in self.participant_proxies.iter() {
      let lease_duration = sp
        .lease_duration
        .unwrap_or(DEFAULT_PARTICIPANT_LEASE_DURATION);
      //let lease_duration = lease_duration + lease_duration; // double it
      match self.participant_last_life_signs.get(&guid) {
        Some(&last_life) => {
          // keep, if duration not exeeded
          let elapsed = Duration::from_std(inow.duration_since(last_life));
          if elapsed <= lease_duration {
            // this is a keeper
          } else {
            info!("participant cleanup - deleting participant proxy {:?}. lease_duration = {:?} elapsed = {:?}",
                  guid, lease_duration, elapsed);
            to_remove.push(guid);
          }
        }
        None => {
          error!("Participant {:?} not in last_life_signs table?", guid);
        }
      } // match
    } // for
    for guid in to_remove.iter() {
      self.remove_participant(*guid);
    }
    to_remove
  }

  fn topic_has_writers_or_readers(&self, topic_name: &str) -> bool {
    // TODO: This entire function has silly implementation.
    // We should really have a separate map from Topic to Readers & Writers
    if self
      .local_topic_readers
      .iter()
      .any(|(_, p)| p.subscription_topic_data.topic_name() == topic_name)
    {
      return true;
    }

    if self
      .local_topic_writers
      .iter()
      .any(|(_, p)| p.publication_topic_data.topic_name == topic_name)
    {
      return true;
    }

    if self
      .external_topic_readers
      .values()
      .any(|p| p.subscription_topic_data.topic_name() == topic_name)
    {
      return true;
    }

    if self
      .external_topic_writers
      .values()
      .any(|p| p.publication_topic_data.topic_name == topic_name)
    {
      return true;
    }

    false
  }

  pub fn topic_cleanup(&mut self) {
    // removing topics that have no readers or writers
    let dead_topics: Vec<_> = self
      .topics
      .iter()
      .map(|(tn, _)| tn)
      .filter(|tn| !self.topic_has_writers_or_readers(tn))
      .cloned()
      .collect();
    for dt in dead_topics.iter() {
      self.topics.remove(dt);
    }
  }

  pub fn get_participants(&self) -> impl Iterator<Item = &SpdpDiscoveredParticipantData> {
    self.participant_proxies.values()
  }

  pub fn update_local_topic_writer(&mut self, writer: DiscoveredWriterData) {
    self
      .local_topic_writers
      .insert(writer.writer_proxy.remote_writer_guid, writer);
    self.writers_updated = true;
  }

  pub fn remove_local_topic_writer(&mut self, guid: GUID) {
    self.local_topic_writers.remove(&guid);
    self.writers_updated = true;
  }

  pub fn get_external_reader_proxies<'a>(
    &'a self,
  ) -> impl Iterator<Item = &DiscoveredReaderData> + 'a {
    self.external_topic_readers.values()
  }

  pub fn get_external_writer_proxies<'a>(
    &'a self,
  ) -> impl Iterator<Item = &DiscoveredWriterData> + 'a {
    self.external_topic_writers.values()
  }

  // TODO: This is silly. Returns one of the paramters cloned, or None
  // TODO: Why are we here checking if discovery db already has this? What about
  // reader proxies in writers?
  pub fn update_subscription(
    &mut self,
    data: &DiscoveredReaderData,
  ) -> Option<(DiscoveredReaderData, RtpsReaderProxy)> {
    let guid = data.reader_proxy.remote_reader_guid;
    // we could return None to indicate that we already knew all about this reader
    // To do that, we should check that the reader is the same as what we have in
    // the DB already.
    match self.external_topic_readers.get(&guid) {
      Some(drd) if drd == data => None, // already have this
      _ => {
        self.external_topic_readers.insert(guid, data.clone());
        // fill in the default locators, in case DRD did not provide any
        let default_locator_lists = self
          .find_participant_proxy(guid.guid_prefix)
          .map(|pp| {
            debug!("Added default locators to Reader {:?}", guid);
            (
              pp.default_unicast_locators.clone(),
              pp.default_multicast_locators.clone(),
            )
          })
          .unwrap_or_else(|| {
            if guid.guid_prefix != GuidPrefix::GUIDPREFIX_UNKNOWN {
              // This is normal, since we might not know about the participant yet.
              debug!(
                "No remote participant known for {:?}\nSearched with {:?} in {:?}",
                data,
                guid.guid_prefix,
                self.participant_proxies.keys()
              );
            }
            (Vec::default(), Vec::default())
          });
        debug!("External reader: {:?}", data);
        Some((
          data.clone(),
          RtpsReaderProxy::from_discovered_reader_data(
            data,
            default_locator_lists.0,
            default_locator_lists.1,
          ),
        ))
      }
    }
  }

  // TODO: This is silly. Returns one of the paramters cloned, or None
  pub fn update_publication(
    &mut self,
    data: &DiscoveredWriterData,
  ) -> Option<DiscoveredWriterData> {
    match self
      .external_topic_writers
      .get(&data.writer_proxy.remote_writer_guid)
    {
      Some(dwd) if dwd == data => None, // already up to date
      _ => {
        self
          .external_topic_writers
          .insert(data.writer_proxy.remote_writer_guid, data.clone());
        debug!("External writer: {:?}", data);
        Some(data.clone())
      }
    }
  }

  pub fn update_topic_data_drd(&mut self, drd: &DiscoveredReaderData) {
    let topic_data = DiscoveredTopicData::new(TopicBuiltinTopicData {
      key: None,
      name: drd.subscription_topic_data.topic_name().clone(),
      type_name: drd.subscription_topic_data.type_name().clone(),
      durability: *drd.subscription_topic_data.durability(),
      deadline: *drd.subscription_topic_data.deadline(),
      latency_budget: *drd.subscription_topic_data.latency_budget(),
      liveliness: *drd.subscription_topic_data.liveliness(),
      reliability: *drd.subscription_topic_data.reliability(),
      lifespan: *drd.subscription_topic_data.lifespan(),
      destination_order: *drd.subscription_topic_data.destination_order(),
      presentation: *drd.subscription_topic_data.presentation(),
      history: None,
      resource_limits: None,
      ownership: *drd.subscription_topic_data.ownership(),
    });

    self.update_topic_data(&topic_data);
  }

  pub fn update_topic_data_dwd(&mut self, dwd: &DiscoveredWriterData) {
    let topic_data = DiscoveredTopicData::new(TopicBuiltinTopicData {
      key: None,
      name: dwd.publication_topic_data.topic_name.clone(),
      type_name: dwd.publication_topic_data.type_name.clone(),
      durability: dwd.publication_topic_data.durability,
      deadline: dwd.publication_topic_data.deadline,
      latency_budget: dwd.publication_topic_data.latency_budget,
      liveliness: dwd.publication_topic_data.liveliness,
      reliability: dwd.publication_topic_data.reliability,
      lifespan: dwd.publication_topic_data.lifespan,
      destination_order: dwd.publication_topic_data.destination_order,
      presentation: dwd.publication_topic_data.presentation,
      history: None,
      resource_limits: None,
      ownership: dwd.publication_topic_data.ownership,
    });

    self.update_topic_data(&topic_data);
  }

  pub fn update_topic_data_p(&mut self, topic: &Topic) {
    let topic_data = DiscoveredTopicData::new(TopicBuiltinTopicData {
      key: None,
      name: topic.get_name(),
      type_name: String::from(topic.get_type().name()),
      durability: topic.get_qos().durability,
      deadline: topic.get_qos().deadline,
      latency_budget: topic.get_qos().latency_budget,
      liveliness: topic.get_qos().liveliness,
      reliability: topic.get_qos().reliability,
      lifespan: topic.get_qos().lifespan,
      destination_order: topic.get_qos().destination_order,
      presentation: topic.get_qos().presentation,
      history: topic.get_qos().history,
      resource_limits: topic.get_qos().resource_limits,
      ownership: topic.get_qos().ownership,
    });

    self.update_topic_data(&topic_data);
  }

  pub fn update_topic_data(&mut self, data: &DiscoveredTopicData) -> bool {
    trace!("Update topic data: {:?}", &data);
    let topic_name = data.topic_data.name.clone();

    match self.topics.get_mut(&data.topic_data.name) {
      Some(t) => *t = data.clone(),
      None => {
        self.topics.insert(topic_name, data.clone());
        if let Some(c) = &self.event_sender {
          let _ = c.try_send(());
        }
      }
    };

    true
  }

  // local topic readers
  pub fn update_local_topic_reader(
    &mut self,
    domain_participant: &DomainParticipant,
    topic: &Topic,
    reader: &ReaderIngredients,
  ) {
    let reader_guid = reader.guid;

    let reader_proxy = RtpsReaderProxy::from_reader(
      reader,
      domain_participant.domain_id(),
      domain_participant.participant_id(),
    );

    let mut subscription_data = SubscriptionBuiltinTopicData::new(
      reader_guid,
      topic.get_name(),
      topic.get_type().name().to_string(),
      &topic.get_qos(),
    );
    subscription_data.set_participant_key(domain_participant.get_guid());

    // TODO: possibly change content filter to dynamic value
    let content_filter = None;

    let discovered_reader_data = DiscoveredReaderData {
      reader_proxy: ReaderProxy::from(reader_proxy),
      subscription_topic_data: subscription_data,
      content_filter,
    };

    self
      .local_topic_readers
      .insert(reader_guid, discovered_reader_data);

    self.readers_updated = true;
  }

  pub fn remove_local_topic_reader(&mut self, guid: GUID) {
    self.local_topic_readers.remove(&guid);
    self.readers_updated = true;
  }

  pub fn is_readers_updated(&self) -> bool {
    self.readers_updated
  }

  pub fn readers_updated(&mut self, updated: bool) {
    self.readers_updated = updated;
  }

  pub fn is_writers_updated(&self) -> bool {
    self.writers_updated
  }

  pub fn writers_updated(&mut self, updated: bool) {
    self.writers_updated = updated;
  }

  pub fn get_all_local_topic_readers(&self) -> impl Iterator<Item = &DiscoveredReaderData> {
    self.local_topic_readers.iter().map(|(_, p)| p)
  }

  pub fn get_all_local_topic_writers(&self) -> impl Iterator<Item = &DiscoveredWriterData> {
    self.local_topic_writers.iter().map(|(_, p)| p)
  }

  pub fn get_all_topics(&self) -> impl Iterator<Item = &DiscoveredTopicData> {
    self
      .topics
      .iter()
      .filter(|(s, _)| !s.starts_with("DCPS"))
      .map(|(_, v)| v)
  }

  pub fn get_topic(&self, topic_name: &str) -> Option<&DiscoveredTopicData> {
    self.topics.get(topic_name)
  }

  pub fn new_topic_token() -> mio::Token {
    mio::Token(0)
  }

  // // TODO: return iterator somehow?
  #[cfg(test)] // used only for testing
  pub fn get_local_topic_readers<'a, T: TopicDescription>(
    &'a self,
    topic: &'a T,
  ) -> Vec<&DiscoveredReaderData> {
    let topic_name = topic.get_name();
    self
      .local_topic_readers
      .iter()
      .filter(|(_, p)| *p.subscription_topic_data.topic_name() == topic_name)
      .map(|(_, p)| p)
      .collect()
  }

  pub fn update_lease_duration(&mut self, data: ParticipantMessageData) {
    let now = Instant::now();
    let prefix = data.guid;
    self
      .external_topic_writers
      .range_mut(prefix.range())
      .for_each(|(_guid, p)| p.last_updated = now);
  }
}

#[cfg(test)]

mod tests {
  use std::{
    rc::Rc,
    sync::{Arc, RwLock},
    time::Duration as StdDuration,
  };

  use byteorder::LittleEndian;

  use super::*;
  use crate::{
    dds::{
      qos::QosPolicies, reader::Reader, statusevents::DataReaderStatus, topic::TopicKind,
      with_key::datareader::ReaderCommand,
    },
    network::udp_sender::UDPSender,
    serialization::cdr_serializer::CDRSerializerAdapter,
    structure::{dds_cache::DDSCache, guid::*},
    test::{
      random_data::RandomData,
      test_data::{reader_proxy_data, spdp_participant_data, subscription_builtin_topic_data},
    },
  };

  #[test]
  fn discdb_participant_operations() {
    let mut discoverydb = DiscoveryDB::new(GUID::new_particiapnt_guid(), None);
    let mut data = spdp_participant_data().unwrap();
    data.lease_duration = Some(Duration::from(StdDuration::from_secs(1)));

    discoverydb.update_participant(&data);
    assert!(discoverydb.participant_proxies.len() == 1);

    discoverydb.update_participant(&data);
    assert!(discoverydb.participant_proxies.len() == 1);

    std::thread::sleep(StdDuration::from_secs(2));
    discoverydb.participant_cleanup();
    assert!(discoverydb.participant_proxies.is_empty());

    // TODO: more operations tests
  }

  #[test]
  fn discdb_writer_proxies() {
    let _discoverydb = DiscoveryDB::new(GUID::new_particiapnt_guid(), None);
    let topic_name = String::from("some_topic");
    let type_name = String::from("RandomData");
    let _dreader = DiscoveredReaderData::default(topic_name, type_name);

    // TODO: more tests :)
  }

  #[test]
  fn discdb_subscription_operations() {
    let mut discovery_db = DiscoveryDB::new(GUID::new_particiapnt_guid(), None);

    let domain_participant = DomainParticipant::new(0).expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic(
        "Foobar".to_string(),
        "RandomData".to_string(),
        &QosPolicies::qos_none(),
        TopicKind::WithKey,
      )
      .unwrap();
    let topic2 = domain_participant
      .create_topic(
        "Barfoo".to_string(),
        "RandomData".to_string(),
        &QosPolicies::qos_none(),
        TopicKind::WithKey,
      )
      .unwrap();

    let publisher1 = domain_participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let dw = publisher1
      .create_datawriter::<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>>(
        topic.clone(),
        None,
      )
      .unwrap();

    let writer_data = DiscoveredWriterData::new(&dw, &topic, &domain_participant);

    let _writer_key = writer_data.writer_proxy.remote_writer_guid;
    discovery_db.update_local_topic_writer(writer_data);
    assert_eq!(discovery_db.local_topic_writers.len(), 1);

    let publisher2 = domain_participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let dw2 = publisher2
      .create_datawriter::<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>>(
        topic.clone(),
        None,
      )
      .unwrap();
    let writer_data2 = DiscoveredWriterData::new(&dw2, &topic, &domain_participant);
    let _writer2_key = writer_data2.writer_proxy.remote_writer_guid;
    discovery_db.update_local_topic_writer(writer_data2);
    assert_eq!(discovery_db.local_topic_writers.len(), 2);

    // creating data
    let reader1 = reader_proxy_data().unwrap();
    let mut reader1sub = subscription_builtin_topic_data().unwrap();
    reader1sub.set_key(reader1.remote_reader_guid);
    reader1sub.set_topic_name(&topic.get_name());
    let dreader1 = DiscoveredReaderData {
      reader_proxy: reader1.clone(),
      subscription_topic_data: reader1sub.clone(),
      content_filter: None,
    };
    discovery_db.update_subscription(&dreader1);

    let reader2 = reader_proxy_data().unwrap();
    let mut reader2sub = subscription_builtin_topic_data().unwrap();
    reader2sub.set_key(reader2.remote_reader_guid);
    reader2sub.set_topic_name(&topic2.get_name());
    let dreader2 = DiscoveredReaderData {
      reader_proxy: reader2,
      subscription_topic_data: reader2sub,
      content_filter: None,
    };
    discovery_db.update_subscription(&dreader2);

    let reader3 = reader1;
    let reader3sub = reader1sub.clone();
    let dreader3 = DiscoveredReaderData {
      reader_proxy: reader3,
      subscription_topic_data: reader3sub,
      content_filter: None,
    };
    discovery_db.update_subscription(&dreader3);

    // TODO: there might be a need for different scenarios
  }

  #[test]
  fn discdb_local_topic_reader() {
    let dp = DomainParticipant::new(0).expect("Failed to create participant");
    let topic = dp
      .create_topic(
        "some topic name".to_string(),
        "Wazzup".to_string(),
        &QosPolicies::qos_none(),
        TopicKind::WithKey,
      )
      .unwrap();
    let mut discoverydb = DiscoveryDB::new(GUID::new_particiapnt_guid(), None);

    let (notification_sender, _notification_receiver) = mio_extras::channel::sync_channel(100);
    let (status_sender, _status_receiver) =
      mio_extras::channel::sync_channel::<DataReaderStatus>(100);
    let (_reader_commander1, reader_command_receiver1) =
      mio_extras::channel::sync_channel::<ReaderCommand>(100);
    let (_reader_commander2, reader_command_receiver2) =
      mio_extras::channel::sync_channel::<ReaderCommand>(100);

    let reader_ing = ReaderIngredients {
      guid: GUID::dummy_test_guid(EntityKind::READER_NO_KEY_USER_DEFINED),
      notification_sender: notification_sender.clone(),
      status_sender: status_sender.clone(),
      topic_name: topic.get_name(),
      qos_policy: QosPolicies::qos_none(),
      data_reader_command_receiver: reader_command_receiver1,
    };

    discoverydb.update_local_topic_reader(&dp, &topic, &reader_ing);
    assert_eq!(discoverydb.local_topic_readers.len(), 1);
    assert_eq!(discoverydb.get_local_topic_readers(&topic).len(), 1);

    discoverydb.update_local_topic_reader(&dp, &topic, &reader_ing);
    assert_eq!(discoverydb.local_topic_readers.len(), 1);
    assert_eq!(discoverydb.get_local_topic_readers(&topic).len(), 1);

    let _reader = Reader::new(
      reader_ing,
      Arc::new(RwLock::new(DDSCache::new())),
      Rc::new(UDPSender::new(0).unwrap()),
      mio_extras::timer::Builder::default().build(),
    );

    let reader_ing = ReaderIngredients {
      guid: GUID::new_with_prefix_and_id(
        GuidPrefix::new(b"Another fake"),
        EntityId {
          entity_key: [1, 2, 3],
          entity_kind: EntityKind::READER_NO_KEY_USER_DEFINED,
        },
      ), // GUID needs to be different in order to be added
      notification_sender,
      status_sender,
      topic_name: topic.get_name(),
      qos_policy: QosPolicies::qos_none(),
      data_reader_command_receiver: reader_command_receiver2,
    };

    discoverydb.update_local_topic_reader(&dp, &topic, &reader_ing);
    assert_eq!(discoverydb.get_local_topic_readers(&topic).len(), 2);
    assert_eq!(discoverydb.get_all_local_topic_readers().count(), 2);

    let _reader = Reader::new(
      reader_ing,
      Arc::new(RwLock::new(DDSCache::new())),
      Rc::new(UDPSender::new(0).unwrap()),
      mio_extras::timer::Builder::default().build(),
    );
  }
}
