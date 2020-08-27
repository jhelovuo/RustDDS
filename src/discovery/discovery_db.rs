use std::{
  time::Duration,
  collections::HashMap,
  net::{IpAddr, SocketAddr},
  io::Error,
};

use crate::dds::qos::HasQoSPolicy;

use crate::structure::{
  guid::GUID,
  duration::Duration as SDuration,
  entity::Entity,
  locator::{Locator, LocatorList},
};

use crate::{
  network::constant::{get_user_traffic_unicast_port, get_user_traffic_multicast_port},
  dds::{
    rtps_reader_proxy::RtpsReaderProxy, reader::Reader, participant::DomainParticipant,
    topic::Topic, rtps_writer_proxy::RtpsWriterProxy,
  },
};

use super::data_types::{
  topic_data::{
    SubscriptionBuiltinTopicData, DiscoveredReaderData, ReaderProxy, DiscoveredWriterData,
  },
  spdp_participant_data::SPDPDiscoveredParticipantData,
};

pub struct DiscoveryDB {
  participant_proxies: HashMap<GUID, SPDPDiscoveredParticipantData>,
  // local writer proxies for topics (topic name acts as key)
  local_topic_writers: Vec<DiscoveredWriterData>,
  // local reader proxies for topics (topic name acts as key)
  local_topic_readers: Vec<DiscoveredReaderData>,

  writers_reader_proxies: HashMap<GUID, Vec<DiscoveredReaderData>>,
  readers_writer_proxies: HashMap<GUID, Vec<DiscoveredWriterData>>,
}

impl DiscoveryDB {
  pub fn new() -> DiscoveryDB {
    DiscoveryDB {
      participant_proxies: HashMap::new(),
      local_topic_writers: Vec::new(),
      local_topic_readers: Vec::new(),
      writers_reader_proxies: HashMap::new(),
      readers_writer_proxies: HashMap::new(),
    }
  }

  pub fn update_participant(&mut self, data: &SPDPDiscoveredParticipantData) -> bool {
    match data.participant_guid {
      Some(guid) => {
        self.participant_proxies.insert(guid, data.clone());
        true
      }
      _ => false,
    }
  }

  pub fn participant_cleanup(&mut self) {
    let instant = time::precise_time_ns();
    self.participant_proxies.retain(|_, d| {
      // TODO: do we always assume that lease duration exists?
      SDuration::from(Duration::from_nanos(instant - d.updated_time))
        < *d.lease_duration.as_ref().unwrap()
    });
  }

  // TODO: collecting participants might be slowish, might want to consider returning just iterator
  pub fn get_participants(&self) -> Vec<&SPDPDiscoveredParticipantData> {
    self.participant_proxies.iter().map(|(_, p)| p).collect()
  }

  pub fn update_local_topic_writer(&mut self, writer: DiscoveredWriterData) {
    let dbwriter = self
      .local_topic_writers
      .iter_mut()
      .find(|p| p.writer_proxy.remote_writer_guid == writer.writer_proxy.remote_writer_guid);

    match dbwriter {
      Some(v) => {
        *v = writer;
        return;
      }
      None => self.local_topic_writers.push(writer),
    };
  }

  pub fn get_writers_reader_proxies(&self, guid: &GUID) -> Option<Vec<&DiscoveredReaderData>> {
    self
      .writers_reader_proxies
      .get(guid)
      .map(|p| p.iter().map(|c| c).collect())
  }

  pub fn get_readers_writer_proxies(&self, guid: &GUID) -> Option<Vec<&DiscoveredWriterData>> {
    self
      .readers_writer_proxies
      .get(guid)
      .map(|p| p.iter().map(|c| c).collect())
  }

  pub fn update_subscription(&mut self, data: &DiscoveredReaderData) -> bool {
    let topic_name = match data.subscription_topic_data.topic_name.as_ref() {
      Some(v) => v,
      None => {
        println!("Failed to update subscription. No topic name.");
        return false;
      }
    };

    let writers: Vec<DiscoveredWriterData> = self
      .local_topic_writers
      .iter()
      .filter(|p| {
        let ptopic_name = p.publication_topic_data.topic_name.as_ref();
        match ptopic_name {
          Some(tn) => *tn == *topic_name,
          None => false,
        }
      })
      .map(|p| p.clone())
      .collect();

    let rtps_reader_proxy = match RtpsReaderProxy::from(&data) {
      Some(v) => v,
      None => {
        println!("Failed to update subscription. Cannot parse RtpsReaderProxy.");
        return false;
      }
    };

    for writer in writers.into_iter() {
      let guid = match &writer.writer_proxy.remote_writer_guid {
        Some(guid) => guid,
        None => {
          println!("warning: Writer doesn't have GUID.");
          continue;
        }
      };

      // update writers reader proxy
      self.update_writers_reader_proxy(guid, data.clone());

      // update writers local data
      let wrp = self.writers_reader_proxies.get_mut(guid);

      match wrp {
        Some(v) => v
          .iter_mut()
          // TODO: handle unwrap
          .filter(|p| {
            p.reader_proxy.remote_reader_guid.unwrap() == rtps_reader_proxy.remote_reader_guid
          })
          .for_each(|p| p.update(&rtps_reader_proxy)),
        None => {
          self.writers_reader_proxies.insert(
            // TODO: handle unwrap
            guid.clone(),
            vec![data.clone()],
          );
          continue;
        }
      };
    }

    true
  }

  pub fn update_publication(&mut self, data: &DiscoveredWriterData) -> bool {
    let topic_name = match data.publication_topic_data.topic_name.as_ref() {
      Some(v) => v,
      None => {
        println!("Failed to update publication. No topic name.");
        return false;
      }
    };

    let readers: Vec<DiscoveredReaderData> = self
      .local_topic_readers
      .iter()
      .filter(|p| {
        let ptopic_name = p.subscription_topic_data.topic_name.as_ref();
        match ptopic_name {
          Some(tn) => *tn == *topic_name,
          None => false,
        }
      })
      .map(|p| p.clone())
      .collect();

    let rtps_writer_proxy = match RtpsWriterProxy::from(&data) {
      Some(v) => v,
      None => {
        println!("Failed to update publication. Cannot parse RtpsWriterProxy.");
        return false;
      }
    };

    for reader in readers.into_iter() {
      let guid = match &reader.reader_proxy.remote_reader_guid {
        Some(guid) => guid,
        None => {
          println!("warning: Reader doesn't have GUID");
          continue;
        }
      };

      // update reader writer proxy
      self.update_readers_writer_proxy(guid, data.clone());

      let rwp = self.readers_writer_proxies.get_mut(guid);

      match rwp {
        Some(v) => v
          .iter_mut()
          .filter(|p| match p.writer_proxy.remote_writer_guid {
            Some(g) => g == rtps_writer_proxy.remote_writer_guid,
            None => false,
          })
          .for_each(|p| p.update(&rtps_writer_proxy)),
        None => {
          self
            .readers_writer_proxies
            .insert(guid.clone(), vec![data.clone()]);
          continue;
        }
      };
    }

    true
  }

  pub fn update_writers_reader_proxy(
    &mut self,
    writer: &GUID,
    reader: DiscoveredReaderData,
  ) -> bool {
    let mut proxies = match self.writers_reader_proxies.get(&writer) {
      Some(prox) => prox.clone(),
      None => Vec::new(),
    };

    let guid = match &reader.reader_proxy.remote_reader_guid {
      Some(g) => g,
      None => {
        println!("Failed to update writers reader proxy. No reader guid.");
        return false;
      }
    };

    let value: Option<usize> =
      proxies
        .iter()
        .position(|x| match x.reader_proxy.remote_reader_guid {
          Some(xguid) => xguid == *guid,
          None => false,
        });
    match value {
      Some(val) => proxies[val] = reader,
      None => proxies.push(reader),
    };

    self.writers_reader_proxies.insert(writer.clone(), proxies);
    true
  }

  pub fn update_readers_writer_proxy(
    &mut self,
    reader: &GUID,
    writer: DiscoveredWriterData,
  ) -> bool {
    let mut proxies = match self.readers_writer_proxies.get(&reader) {
      Some(prox) => prox.clone(),
      None => Vec::new(),
    };

    let guid = match &writer.writer_proxy.remote_writer_guid {
      Some(g) => g,
      None => {
        println!("Failed to update readers writer proxy. No writer guid.");
        return false;
      }
    };

    let value: Option<usize> =
      proxies
        .iter()
        .position(|x| match x.writer_proxy.remote_writer_guid {
          Some(xguid) => xguid == *guid,
          None => false,
        });

    match value {
      Some(val) => proxies[val] = writer,
      None => proxies.push(writer),
    };

    self.readers_writer_proxies.insert(reader.clone(), proxies);
    true
  }

  // local topic readers
  pub fn update_local_topic_reader(
    &mut self,
    domain_participant: &DomainParticipant,
    topic: &Topic,
    reader: &Reader,
  ) {
    let mut reader_proxy = RtpsReaderProxy::new(reader.get_guid().clone());
    let local_ips: Result<Vec<IpAddr>, Error> = get_if_addrs::get_if_addrs().map(|p| {
      p.iter()
        .filter(|ip| !ip.is_loopback())
        .map(|ip| ip.ip())
        .collect()
    });

    let unicast_locators: LocatorList = match local_ips {
      Ok(ips) => ips
        .iter()
        .map(|p| {
          Locator::from(SocketAddr::new(
            p.clone(),
            get_user_traffic_unicast_port(
              domain_participant.domain_id(),
              domain_participant.participant_id(),
            ),
          ))
        })
        .collect(),
      _ => return,
    };

    let multicast_locators: LocatorList = vec![Locator::from(SocketAddr::new(
      "239.255.0.1".parse().unwrap(),
      get_user_traffic_multicast_port(domain_participant.domain_id()),
    ))];

    // let unicast_socketaddr = SocketAddr::new("ip", port)
    reader_proxy.unicast_locator_list = unicast_locators;
    reader_proxy.multicast_locator_list = multicast_locators;
    // TODO: remove these constant evaluations
    reader_proxy.expects_in_line_qos = false;
    reader_proxy.is_active = true;

    let subscription_data = SubscriptionBuiltinTopicData {
      key: Some(reader.get_guid().clone()),
      participant_key: Some(domain_participant.get_guid().clone()),
      topic_name: Some(topic.get_name().to_string()),
      type_name: Some(topic.get_type().name().to_string()),
      durability: topic.get_qos().durability.clone(),
      deadline: topic.get_qos().deadline.clone(),
      latency_budget: topic.get_qos().latency_budget.clone(),
      liveliness: topic.get_qos().liveliness.clone(),
      reliability: topic.get_qos().reliability.clone(),
      ownership: topic.get_qos().ownership.clone(),
      destination_order: topic.get_qos().destination_order.clone(),
      time_based_filter: topic.get_qos().time_based_filter.clone(),
      presentation: topic.get_qos().presentation.clone(),
      lifespan: topic.get_qos().lifespan.clone(),
    };

    // TODO: possibly change content filter to dynamic value
    let content_filter = None;

    let discovered_reader_data = DiscoveredReaderData {
      reader_proxy: ReaderProxy::from(reader_proxy),
      subscription_topic_data: subscription_data,
      content_filter,
    };

    // updating local topic readers
    let mut treaders: Vec<&mut DiscoveredReaderData> = self
      .local_topic_readers
      .iter_mut()
      .filter(|p| {
        *p.subscription_topic_data.topic_name.as_ref().unwrap() == String::from(topic.get_name())
          && *p.reader_proxy.remote_reader_guid.as_ref().unwrap() == *reader.get_guid()
      })
      .collect();

    if !treaders.is_empty() {
      treaders
        .iter_mut()
        .filter(|p| {
          p.reader_proxy.remote_reader_guid.unwrap()
            == discovered_reader_data
              .reader_proxy
              .remote_reader_guid
              .unwrap()
        })
        .for_each(|p| **p = discovered_reader_data.clone());
    } else {
      self.local_topic_readers.push(discovered_reader_data);
    }
  }

  pub fn get_all_local_topic_readers(&self) -> Vec<&DiscoveredReaderData> {
    self.local_topic_readers.iter().collect()
  }

  pub fn get_all_local_topic_writers(&self) -> Vec<&DiscoveredWriterData> {
    self.local_topic_writers.iter().collect()
  }

  // TODO: maybe query parameter should only be topics' name
  pub fn get_local_topic_readers(&self, topic: &Topic) -> Vec<&DiscoveredReaderData> {
    let topic_name = topic.get_name().to_string();
    self
      .local_topic_readers
      .iter()
      .filter(|p| *p.subscription_topic_data.topic_name.as_ref().unwrap() == topic_name)
      .collect()
  }
}

#[cfg(test)]

mod tests {
  use super::*;

  use crate::{
    structure::dds_cache::DDSCache,
    test::{
      random_data::RandomData,
      test_data::{subscription_builtin_topic_data, spdp_participant_data, reader_proxy_data},
    },
    dds::{qos::QosPolicies, typedesc::TypeDesc},
  };
  use std::sync::{RwLock, Arc};

  use crate::structure::guid::*;
  use crate::serialization::cdrSerializer::CDR_serializer_adapter;
  use byteorder::LittleEndian;

  #[test]
  fn discdb_participant_operations() {
    let mut discoverydb = DiscoveryDB::new();
    let mut data = spdp_participant_data().unwrap();
    data.lease_duration = Some(SDuration::from(Duration::from_secs(1)));

    discoverydb.update_participant(&data);
    assert!(discoverydb.participant_proxies.len() == 1);

    discoverydb.update_participant(&data);
    assert!(discoverydb.participant_proxies.len() == 1);

    std::thread::sleep(Duration::from_secs(2));
    discoverydb.participant_cleanup();
    assert!(discoverydb.participant_proxies.len() == 0);

    // TODO: more operations tests
  }

  #[test]
  fn discdb_writer_proxies() {
    let mut discoverydb = DiscoveryDB::new();
    let topic_name = String::from("some_topic");
    let type_name = String::from("RandomData");
    let dreader = DiscoveredReaderData::default(&topic_name, &type_name);
    discoverydb.update_writers_reader_proxy(&GUID::new(), dreader);
    assert_eq!(discoverydb.writers_reader_proxies.len(), 1);

    // TODO: more tests :)
  }

  #[test]
  fn discdb_subscription_operations() {
    let mut discovery_db = DiscoveryDB::new();

    let domain_participant = DomainParticipant::new(16, 0);
    let topic = domain_participant
      .create_topic(
        "Foobar",
        TypeDesc::new(String::from("RandomData")),
        &QosPolicies::qos_none(),
      )
      .unwrap();
    let topic2 = domain_participant
      .create_topic(
        "Barfoo",
        TypeDesc::new(String::from("RandomData")),
        &QosPolicies::qos_none(),
      )
      .unwrap();

    let publisher1 = domain_participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let dw = publisher1
      .create_datawriter::<RandomData, CDR_serializer_adapter<RandomData,LittleEndian>>(None, &topic, &QosPolicies::qos_none())
      .unwrap();

    let writer_data = DiscoveredWriterData::new(&dw, &topic, &domain_participant);

    let writer_key = writer_data.writer_proxy.remote_writer_guid.unwrap().clone();
    discovery_db.update_local_topic_writer(writer_data);
    assert_eq!(discovery_db.local_topic_writers.len(), 1);

    let publisher2 = domain_participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let dw2 = publisher2
      .create_datawriter::<RandomData,CDR_serializer_adapter<RandomData,LittleEndian>>(None, &topic, &QosPolicies::qos_none())
      .unwrap();
    let writer_data2 = DiscoveredWriterData::new(&dw2, &topic, &domain_participant);
    let _writer2_key = writer_data2
      .writer_proxy
      .remote_writer_guid
      .unwrap()
      .clone();
    discovery_db.update_local_topic_writer(writer_data2);
    assert_eq!(discovery_db.local_topic_writers.len(), 2);

    // creating data
    let reader1 = reader_proxy_data().unwrap();
    let mut reader1sub = subscription_builtin_topic_data().unwrap();
    reader1sub.key = reader1.remote_reader_guid.clone();
    reader1sub.topic_name = Some(topic.get_name().to_string().clone());
    let dreader1 = DiscoveredReaderData {
      reader_proxy: reader1.clone(),
      subscription_topic_data: reader1sub.clone(),
      content_filter: None,
    };
    discovery_db.update_subscription(&dreader1);
    assert_eq!(
      discovery_db
        .writers_reader_proxies
        .get(&writer_key)
        .unwrap()
        .len(),
      1
    );

    let reader2 = reader_proxy_data().unwrap();
    let mut reader2sub = subscription_builtin_topic_data().unwrap();
    reader2sub.key = reader2.remote_reader_guid.clone();
    reader2sub.topic_name = Some(topic2.get_name().to_string().clone());
    let dreader2 = DiscoveredReaderData {
      reader_proxy: reader2,
      subscription_topic_data: reader2sub,
      content_filter: None,
    };
    discovery_db.update_subscription(&dreader2);
    assert_eq!(
      discovery_db
        .writers_reader_proxies
        .get(&writer_key)
        .unwrap()
        .len(),
      1
    );
    let writers_size: usize = discovery_db
      .writers_reader_proxies
      .values()
      .map(|p| p.len())
      .sum();
    assert_eq!(writers_size, 2);

    let reader3 = reader1.clone();
    let reader3sub = reader1sub.clone();
    let dreader3 = DiscoveredReaderData {
      reader_proxy: reader3,
      subscription_topic_data: reader3sub,
      content_filter: None,
    };
    discovery_db.update_subscription(&dreader3);
    assert_eq!(
      discovery_db
        .writers_reader_proxies
        .get(&writer_key)
        .unwrap()
        .len(),
      1
    );

    // TODO: there might be a need for different scenarios
  }

  #[test]
  fn discdb_local_topic_reader() {
    let dp = DomainParticipant::new(12, 0);
    let topic = dp
      .create_topic(
        "some topic name",
        TypeDesc::new(String::from("Wazzup")),
        &QosPolicies::qos_none(),
      )
      .unwrap();
    let mut discoverydb = DiscoveryDB::new();

    let (notification_sender, _notification_receiver) = mio_extras::channel::sync_channel(100);
    let reader = Reader::new(
      GUID::new(),
      notification_sender.clone(),
      Arc::new(RwLock::new(DDSCache::new())),
      topic.get_name().to_string(),
    );

    discoverydb.update_local_topic_reader(&dp, &topic, &reader);
    assert_eq!(discoverydb.local_topic_readers.len(), 1);
    assert_eq!(discoverydb.get_local_topic_readers(&topic).len(), 1);

    discoverydb.update_local_topic_reader(&dp, &topic, &reader);
    assert_eq!(discoverydb.local_topic_readers.len(), 1);
    assert_eq!(discoverydb.get_local_topic_readers(&topic).len(), 1);

    let reader = Reader::new(
      GUID::new(),
      notification_sender.clone(),
      Arc::new(RwLock::new(DDSCache::new())),
      topic.get_name().to_string(),
    );

    discoverydb.update_local_topic_reader(&dp, &topic, &reader);
    assert_eq!(discoverydb.get_local_topic_readers(&topic).len(), 2);
    assert_eq!(discoverydb.get_all_local_topic_readers().len(), 2);
  }
}
