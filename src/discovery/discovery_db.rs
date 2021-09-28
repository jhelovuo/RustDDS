use std::{
  collections::{btree_map::Iter as BTreeIter, HashMap, BTreeMap},
  iter::Map,
  time::Instant,
};

#[allow(unused_imports)]
use log::{error,warn,debug,trace,info};

use crate::{
  dds::qos::HasQoSPolicy, 
  structure::guid::GuidPrefix,
};

use crate::structure::{guid::GUID, guid::EntityId,
  duration::Duration, entity::RTPSEntity, locator::LocatorList};

use crate::{
  dds::{
    rtps_reader_proxy::RtpsReaderProxy, reader::{ReaderIngredients,}, 
    participant::DomainParticipant,
    topic::Topic, traits::TopicDescription,
  },
};

use super::{
  data_types::{
    spdp_participant_data::SPDPDiscoveredParticipantData,
    topic_data::{
      DiscoveredReaderData, DiscoveredTopicData, DiscoveredWriterData, ParticipantMessageData,
      ReaderProxy, SubscriptionBuiltinTopicData, TopicBuiltinTopicData,
    },
  },
};

// If remote participant does not specifiy lease duration, how long silence
// until we pronounce it dead.
const DEFAULT_PARTICIPANT_LEASE_DURATION : Duration = Duration::from_secs(60);

// How much longer to wait than lease duration before pronouncing lost.
const PARTICIPANT_LEASE_DURATION_TOLREANCE : Duration = Duration::from_secs(0);


pub(crate) struct DiscoveryDB {
  my_guid: GUID,
  participant_proxies: BTreeMap<GuidPrefix, SPDPDiscoveredParticipantData>,
  participant_last_life_signs: HashMap<GuidPrefix, Instant>,
  // local writer proxies for topics (topic name acts as key)
  local_topic_writers: HashMap<GUID, DiscoveredWriterData>,
  // local reader proxies for topics (topic name acts as key)
  local_topic_readers: HashMap<GUID, DiscoveredReaderData>,

  // remote readers and writers (via discovery)
  external_topic_readers: BTreeMap<GUID,DiscoveredReaderData>,
  external_topic_writers: BTreeMap<GUID,DiscoveredWriterData>,

  topics: HashMap<String, DiscoveredTopicData>,

  readers_updated: bool,
  writers_updated: bool,
}

impl DiscoveryDB {
  pub fn new(my_guid:GUID) -> DiscoveryDB {
    DiscoveryDB {
      my_guid,
      participant_proxies: BTreeMap::new(),
      participant_last_life_signs: HashMap::new(),
      local_topic_writers: HashMap::new(),
      local_topic_readers: HashMap::new(),
      external_topic_readers: BTreeMap::new(),
      external_topic_writers: BTreeMap::new(),
      topics: HashMap::new(),
      readers_updated: false,
      writers_updated: false,
    }
  }

  pub fn update_participant(&mut self, data: &SPDPDiscoveredParticipantData) {
      debug!("update_participant: {:?}",&data);
      let guid = data.participant_guid;
      // sanity check
      if guid.entityId != EntityId::ENTITYID_PARTICIPANT {
        error!("Discovered participant GUID entity_id is not for participant: {:?}",guid);
        return // Maybe we should discard the participant here?
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

      if self.participant_proxies.get( &guid.guidPrefix ).is_none() {
        info!("New remote participant: {:?}", &data);
        if guid == self.my_guid {
          info!("Remote participant {:?} is myself, but some reflection is good.",
            guid);
        }
      }
      // actual work here:
      self.participant_proxies.insert(guid.guidPrefix, data.clone());
      self.participant_last_life_signs.insert(guid.guidPrefix, Instant::now() );
  }

  pub fn remove_participant(&mut self, guid_prefix: GuidPrefix) {
    info!("removing participant {:?}",guid_prefix);
    self.participant_proxies.remove(&guid_prefix);
    self.participant_last_life_signs.remove(&guid_prefix);

    self.remove_topic_reader_with_prefix(guid_prefix);

    self.remove_topic_writer_with_prefix(guid_prefix);
  }

  pub fn find_participant_proxy(&self, guid_prefix: GuidPrefix) 
    -> Option<&SPDPDiscoveredParticipantData> 
  {
    self.participant_proxies.get( &guid_prefix )
  }

  pub fn find_remote_reader(&self, guid:GUID) -> Option<&DiscoveredReaderData>{
    self.external_topic_readers.get(&guid)
  }

  fn remove_topic_reader_with_prefix(&mut self, guid_prefix: GuidPrefix) {
    // TODO: Implement this using .drain_filter() in BTreeMap once it lands in stable.
    let to_remove :Vec<GUID> = 
          self.external_topic_readers
            .range( guid_prefix.range() )
            .map(|(g,_)| g.clone() )
            .collect();
    for guid in to_remove {
      self.external_topic_readers.remove(&guid);
    }
  }

  pub fn remove_topic_reader(&mut self, guid: GUID) {
    self.external_topic_readers.remove(&guid);
  }

  fn remove_topic_writer_with_prefix(&mut self, guid_prefix: GuidPrefix) {
    // TODO: Implement this using .drain_filter() in BTreeMap once it lands in stable.
    let to_remove :Vec<GUID> = 
          self.external_topic_writers
            .range( guid_prefix.range() )
            .map(|(g,_)| g.clone() )
            .collect();
    for guid in to_remove {
      self.external_topic_writers.remove(&guid);
    }
  }

  pub fn remove_topic_writer(&mut self, guid: GUID) {
    self.external_topic_writers.remove(&guid);
  }


  // Delete participant proxies, if we have not heard of them within lease_duration
  pub fn participant_cleanup(&mut self) -> Vec<GuidPrefix> {
    let inow = Instant::now();

    let mut to_remove = Vec::new();
    // TODO: We are not cleaning up liast_life_signs table, but that should not be a problem,
    // except for a slight memory leak.
    for (&guid,sp) in self.participant_proxies.iter() {
      let lease_duration = sp.lease_duration
            .unwrap_or( DEFAULT_PARTICIPANT_LEASE_DURATION );
      //let lease_duration = lease_duration + lease_duration; // double it
      match self.participant_last_life_signs.get(&guid) {
        Some(&last_life) => {
          // keep, if duration not exeeded
          let elapsed = Duration::from_std(inow.duration_since(last_life));
          if elapsed <= lease_duration {
            () // this is a keeper
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
    for guid in &to_remove {
      self.participant_proxies.remove(guid).expect("Removing fail 1");
      self.participant_last_life_signs.remove(guid).expect("Removing fail 2");
    }

    to_remove
  }

  fn topic_has_writers_or_readers(&self, topic_name: &String) -> bool {
    // TODO: This entire function has silly implementation.
    // We should really have a separate map from Topic to Readers & Writers
    if let Some(_) =
      self
        .local_topic_readers
        .iter()
        .find(|(_, p)| p.subscription_topic_data.topic_name() == topic_name)
    {
      return true
    }

    if let Some(_) =
      self
        .local_topic_writers
        .iter()
        .find(|(_, p)| &p.publication_topic_data.topic_name == topic_name)
    {
      return true
    }

    if let Some(_) =
      self
        .external_topic_readers
        .values()
        .find(|p| p.subscription_topic_data.topic_name() == topic_name)
    {
      return true
    }

    if let Some(_) =
      self
        .external_topic_writers
        .values()
        .find(|p| &p.publication_topic_data.topic_name == topic_name)
    {
      return true
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
      .map(|tn| tn.clone())
      .collect();
    for dt in dead_topics.iter() {
      self.topics.remove(dt);
    }
  }

  // Please explain how this works.
  pub fn get_participants<'a>(&'a self) -> Map<
    BTreeIter<'a, GuidPrefix, SPDPDiscoveredParticipantData>,
    fn((&GuidPrefix, &'a SPDPDiscoveredParticipantData)) -> &'a SPDPDiscoveredParticipantData,
  > {
    type cvfun<'a> =
      fn((&GuidPrefix, &'a SPDPDiscoveredParticipantData)) -> &'a SPDPDiscoveredParticipantData;
    fn conver<'a>(
      (_, data): (&GuidPrefix, &'a SPDPDiscoveredParticipantData),
    ) -> &'a SPDPDiscoveredParticipantData {
      data
    }
    let cv: cvfun = conver;
    let a = self.participant_proxies.iter().map(cv);
    a
  }

  pub fn update_local_topic_writer(&mut self, writer: DiscoveredWriterData) {
    self.local_topic_writers.insert(writer.writer_proxy.remote_writer_guid, writer);
    self.writers_updated = true;
  }

  pub fn remove_local_topic_writer(&mut self, guid: GUID) {
    self.local_topic_writers.remove(&guid);
    self.writers_updated = true;
  }

  pub fn get_external_reader_proxies<'a>(&'a self) -> impl Iterator<Item = &DiscoveredReaderData> + 'a {
    self.external_topic_readers.values()
  }

  pub fn get_external_writer_proxies<'a>(&'a self) -> impl Iterator<Item = &DiscoveredWriterData> + 'a {
    self.external_topic_writers.values()
  }

  // TODO: This is silly. Returns one of the paramters cloned, or None
  pub fn update_subscription(&mut self, data: &DiscoveredReaderData) -> Option<(DiscoveredReaderData, RtpsReaderProxy)> {
    let guid = data.reader_proxy.remote_reader_guid;
    // we could return None to indicate that we already knew all about this reader
    // To do that, we should check that the reader is the same as what we have in the DB already.
    match self.external_topic_readers.get(&guid) {
      Some(drd) if drd == data  => None, // already have this
      _ => {
        self.external_topic_readers.insert( guid, data.clone() );
        // fill in the default locators, in case DRD did not provide any
        let default_locator_lists = 
          self.find_participant_proxy(guid.guidPrefix)
            .map(|pp| {
              debug!("Added default locators to Reader {:?}", guid);
              ( pp.default_unicast_locators.clone(), 
                pp.default_multicast_locators.clone() ) } )
            .unwrap_or_else( || {
                if guid.guidPrefix != GuidPrefix::GUIDPREFIX_UNKNOWN {
                  // This is normal, since we might not know about the participant yet.
                  debug!("No remote participant known for {:?}\nSearched with {:?} in {:?}"
                    ,data, guid.guidPrefix, self.participant_proxies.keys() );
                }
                (LocatorList::new(), LocatorList::new()) 
              } );
        debug!("External reader: {:?}",data);
        Some( ( data.clone(), 
                RtpsReaderProxy::from_discovered_reader_data(data, 
                  default_locator_lists.0 , default_locator_lists.1 )))
      }
    }
  }

  // TODO: This is silly. Returns one of the paramters cloned, or None
  pub fn update_publication(&mut self, data: &DiscoveredWriterData) -> Option<DiscoveredWriterData> {
    match self.external_topic_writers.get(&data.writer_proxy.remote_writer_guid) {
      Some(dwd) if dwd == data => None , // already up to date
      _ => {
        self.external_topic_writers.insert(data.writer_proxy.remote_writer_guid, data.clone());
        debug!("External writer: {:?}",data);
        Some(data.clone() )
      }
    }
    
  }

  pub fn update_topic_data_drd(&mut self, drd: &DiscoveredReaderData) {
    let topic_data = DiscoveredTopicData::new(TopicBuiltinTopicData {
      key: None,
      name: drd.subscription_topic_data.topic_name().clone(),
      type_name: drd.subscription_topic_data.type_name().clone(),
      durability: drd.subscription_topic_data.durability().clone(),
      deadline: drd.subscription_topic_data.deadline().clone(),
      latency_budget: drd.subscription_topic_data.latency_budget().clone(),
      liveliness: drd.subscription_topic_data.liveliness().clone(),
      reliability: drd.subscription_topic_data.reliability().clone(),
      lifespan: drd.subscription_topic_data.lifespan().clone(),
      destination_order: drd.subscription_topic_data.destination_order().clone(),
      presentation: drd.subscription_topic_data.presentation().clone(),
      history: None,
      resource_limits: None,
      ownership: drd.subscription_topic_data.ownership().clone(),
    });

    self.update_topic_data(&topic_data);
  }

  pub fn update_topic_data_dwd(&mut self, dwd: &DiscoveredWriterData) {
    let topic_data = DiscoveredTopicData::new(TopicBuiltinTopicData {
      key: None,
      name: dwd.publication_topic_data.topic_name.clone(),
      type_name: dwd.publication_topic_data.type_name.clone(),
      durability: dwd.publication_topic_data.durability.clone(),
      deadline: dwd.publication_topic_data.deadline.clone(),
      latency_budget: dwd.publication_topic_data.latency_budget.clone(),
      liveliness: dwd.publication_topic_data.liveliness.clone(),
      reliability: dwd.publication_topic_data.reliability.clone(),
      lifespan: dwd.publication_topic_data.lifespan.clone(),
      destination_order: dwd.publication_topic_data.destination_order.clone(),
      presentation: dwd.publication_topic_data.presentation.clone(),
      history: None,
      resource_limits: None,
      ownership: dwd.publication_topic_data.ownership.clone(),
    });

    self.update_topic_data(&topic_data);
  }

  pub fn update_topic_data_p(&mut self, topic: &Topic) {
    let topic_data = DiscoveredTopicData::new(TopicBuiltinTopicData {
      key: None,
      name: String::from(topic.get_name()),
      type_name: String::from(topic.get_type().name()),
      durability: topic.get_qos().durability.clone(),
      deadline: topic.get_qos().deadline.clone(),
      latency_budget: topic.get_qos().latency_budget.clone(),
      liveliness: topic.get_qos().liveliness.clone(),
      reliability: topic.get_qos().reliability.clone(),
      lifespan: topic.get_qos().lifespan.clone(),
      destination_order: topic.get_qos().destination_order.clone(),
      presentation: topic.get_qos().presentation.clone(),
      history: topic.get_qos().history.clone(),
      resource_limits: topic.get_qos().resource_limits.clone(),
      ownership: topic.get_qos().ownership.clone(),
    });

    self.update_topic_data(&topic_data);
  }

  pub fn update_topic_data(&mut self, data: &DiscoveredTopicData) -> bool {
    trace!("Update topic data: {:?}",&data);
    let topic_name = data.topic_data.name.clone();

    match self.topics.get_mut(&data.topic_data.name) {
      Some(t) => *t = data.clone(),
      None => {
        self.topics.insert(topic_name, data.clone());
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
      &topic.get_name(),
      &topic.get_type().name(),
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

  pub fn get_all_local_topic_readers<'a>(
    &'a self,
  ) -> impl Iterator<Item = &'a DiscoveredReaderData> {
    self.local_topic_readers.iter().map(|(_, p)| p)
  }

  pub fn get_all_local_topic_writers<'a>(
    &'a self,
  ) -> impl Iterator<Item = &'a DiscoveredWriterData> {
    self.local_topic_writers.iter().map(|(_, p)| p)
  }

  pub fn get_all_topics<'a>(&'a self) -> impl Iterator<Item = &'a DiscoveredTopicData> {
    self.topics.iter()
      .filter(|(s, _)| !s.starts_with("DCPS"))
      .map(|(_, v)| v)
  }

  // // TODO: return iterator somehow?
  #[cfg(test)] // used only for testing
  pub fn get_local_topic_readers<'a, T: TopicDescription>(
    &'a self,
    topic: &'a T,
  ) -> Vec<&DiscoveredReaderData> {
    let topic_name = String::from(topic.get_name());
    self
      .local_topic_readers
      .iter()
      .filter(|(_, p)| {
        *p.subscription_topic_data.topic_name() == topic_name
      })
      .map(|(_, p)| p)
      .collect()
  }

  pub fn update_lease_duration(&mut self, data: ParticipantMessageData) {
    let now = Instant::now();
    let prefix = data.guid;
    self
      .external_topic_writers
      .range_mut( prefix.range() )
      .for_each(|(_guid,p)| p.last_updated = now);
  }
}

#[cfg(test)]

mod tests {
  use super::*;

  use crate::{
    dds::qos::QosPolicies,
    structure::dds_cache::DDSCache,
    dds::topic::TopicKind,
    test::{
      random_data::RandomData,
      test_data::{subscription_builtin_topic_data, spdp_participant_data, reader_proxy_data},
    },
  };
  use std::sync::{RwLock, Arc};

  use crate::structure::guid::*;
  use crate::serialization::cdr_serializer::CDRSerializerAdapter;
  use byteorder::LittleEndian;
  use std::time::Duration as StdDuration;
  use crate::dds::statusevents::DataReaderStatus;
  use crate::dds::with_key::datareader::ReaderCommand;

  #[test]
  fn discdb_participant_operations() {
    let mut discoverydb = DiscoveryDB::new(GUID::new_particiapnt_guid());
    let mut data = spdp_participant_data().unwrap();
    data.lease_duration = Some(Duration::from(StdDuration::from_secs(1)));

    discoverydb.update_participant(&data);
    assert!(discoverydb.participant_proxies.len() == 1);

    discoverydb.update_participant(&data);
    assert!(discoverydb.participant_proxies.len() == 1);

    std::thread::sleep(StdDuration::from_secs(2));
    discoverydb.participant_cleanup();
    assert!(discoverydb.participant_proxies.len() == 0);

    // TODO: more operations tests
  }

  #[test]
  fn discdb_writer_proxies() {
    let _discoverydb = DiscoveryDB::new(GUID::new_particiapnt_guid());
    let topic_name = String::from("some_topic");
    let type_name = String::from("RandomData");
    let _dreader = DiscoveredReaderData::default(&topic_name, &type_name);

    // TODO: more tests :)
  }

  #[test]
  fn discdb_subscription_operations() {
    let mut discovery_db = DiscoveryDB::new(GUID::new_particiapnt_guid());

    let domain_participant = DomainParticipant::new(0).expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic(
        "Foobar",
        "RandomData",
        &QosPolicies::qos_none(),
        TopicKind::WithKey,
      )
      .unwrap();
    let topic2 = domain_participant
      .create_topic(
        "Barfoo",
        "RandomData",
        &QosPolicies::qos_none(),
        TopicKind::WithKey,
      )
      .unwrap();

    let publisher1 = domain_participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let dw = publisher1
      .create_datawriter::<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>>(
        topic.clone(), None,
      )
      .unwrap();

    let writer_data = DiscoveredWriterData::new(&dw, &topic, &domain_participant);

    let _writer_key = writer_data.writer_proxy.remote_writer_guid.clone();
    discovery_db.update_local_topic_writer(writer_data);
    assert_eq!(discovery_db.local_topic_writers.len(), 1);

    let publisher2 = domain_participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let dw2 = publisher2
      .create_datawriter::<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>>(
       topic.clone(), None,
      )
      .unwrap();
    let writer_data2 = DiscoveredWriterData::new(&dw2, &topic, &domain_participant);
    let _writer2_key = writer_data2
      .writer_proxy
      .remote_writer_guid
      .clone();
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

    let reader3 = reader1.clone();
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
        "some topic name",
        "Wazzup",
        &QosPolicies::qos_none(),
        TopicKind::WithKey,
      )
      .unwrap();
    let mut discoverydb = DiscoveryDB::new(GUID::new_particiapnt_guid());

    let (notification_sender, _notification_receiver) = mio_extras::channel::sync_channel(100);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<DataReaderStatus>(100);
    let (_reader_commander1, reader_command_receiver1) =
      mio_extras::channel::sync_channel::<ReaderCommand>(100);
    let (_reader_commander2, reader_command_receiver2) =
      mio_extras::channel::sync_channel::<ReaderCommand>(100);

    let reader = Reader::new(
      GUID::dummy_test_guid(EntityKind::READER_NO_KEY_USER_DEFINED),
      notification_sender.clone(),
      status_sender.clone(),
      Arc::new(RwLock::new(DDSCache::new())),
      topic.get_name().to_string(),
      QosPolicies::qos_none(),
      reader_command_receiver1,
    );

    discoverydb.update_local_topic_reader(&dp, &topic, &reader);
    assert_eq!(discoverydb.local_topic_readers.len(), 1);
    assert_eq!(discoverydb.get_local_topic_readers(&topic).len(), 1);

    discoverydb.update_local_topic_reader(&dp, &topic, &reader);
    assert_eq!(discoverydb.local_topic_readers.len(), 1);
    assert_eq!(discoverydb.get_local_topic_readers(&topic).len(), 1);

    let reader = Reader::new(
      GUID::new_with_prefix_and_id(GuidPrefix::new(b"Another fake"), EntityId {
        entityKey: [1, 2, 3],
        entityKind: EntityKind::READER_NO_KEY_USER_DEFINED
      }), // GUID needs to be different in order to be added
      notification_sender.clone(),
      status_sender.clone(),
      Arc::new(RwLock::new(DDSCache::new())),
      topic.get_name().to_string(),
      QosPolicies::qos_none(),
      reader_command_receiver2,
    );

    discoverydb.update_local_topic_reader(&dp, &topic, &reader);
    assert_eq!(discoverydb.get_local_topic_readers(&topic).len(), 2);
    assert_eq!(discoverydb.get_all_local_topic_readers().count(), 2);
  }
}
