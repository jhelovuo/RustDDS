use std::{time::Duration, collections::HashMap};

use crate::structure::{guid::GUID, duration::Duration as SDuration};

use crate::dds::rtps_reader_proxy::RtpsReaderProxy;
use crate::dds::rtps_writer_proxy::RtpsWriterProxy;
use super::data_types::{
  topic_data::DiscoveredReaderData, spdp_participant_data::SPDPDiscoveredParticipantData,
};

pub struct DiscoveryDB {
  participant_proxies: HashMap<GUID, SPDPDiscoveredParticipantData>,
  // local writer proxies for topics (topic name acts as key)
  local_topic_writers: HashMap<String, Vec<RtpsWriterProxy>>,
  // local reader proxies for topics (topic name acts as key)
  local_topic_readers: HashMap<String, Vec<RtpsReaderProxy>>,

  writers_reader_proxies: HashMap<GUID, Vec<RtpsReaderProxy>>,
  readers_writer_proxies: HashMap<GUID, Vec<RtpsWriterProxy>>,
}

impl DiscoveryDB {
  pub fn new() -> DiscoveryDB {
    DiscoveryDB {
      participant_proxies: HashMap::new(),
      local_topic_writers: HashMap::new(),
      local_topic_readers: HashMap::new(),
      writers_reader_proxies: HashMap::new(),
      readers_writer_proxies: HashMap::new(),
    }
  }

  pub fn add_local_topic_writer(&mut self, topic_name: String, writer: RtpsWriterProxy) -> bool {
    let dbwriter = self.local_topic_writers.get_mut(&topic_name);
    match dbwriter {
      Some(v) => {
        let res = v
          .iter()
          .filter(|p| p.remote_writer_guid == writer.remote_writer_guid)
          .count();
        if res == 0 {
          v.push(writer);
          return true;
        }
        return false;
      }
      None => self.local_topic_writers.insert(topic_name, vec![writer]),
    };
    true
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

  pub fn update_subscription(&mut self, data: &DiscoveredReaderData) -> bool {
    let topic_name = match data.subscription_topic_data.topic_name.as_ref() {
      Some(v) => v,
      None => return false,
    };

    let local_writers = self.local_topic_writers.get(topic_name);
    let writers = match local_writers {
      Some(writers) => writers,
      None => return false,
    };

    let rtps_reader_proxy = match RtpsReaderProxy::from(&data) {
      Some(v) => v,
      None => return false,
    };

    for writer in writers.iter() {
      let wrp = self
        .writers_reader_proxies
        .get_mut(&writer.remote_writer_guid);

      match wrp {
        Some(v) => v
          .iter_mut()
          .filter(|p| p.remote_reader_guid == rtps_reader_proxy.remote_reader_guid)
          .for_each(|p| p.update(&rtps_reader_proxy)),
        None => {
          self
            .writers_reader_proxies
            .insert(writer.remote_writer_guid, vec![rtps_reader_proxy.clone()]);
          continue;
        }
      };
    }

    true
  }

  pub fn participant_cleanup(&mut self) {
    let instant = time::precise_time_ns();
    self.participant_proxies.retain(|_, d| {
      // TODO: do we always assume that lease duration exists?
      SDuration::from(Duration::from_nanos(instant - d.updated_time))
        < *d.lease_duration.as_ref().unwrap()
    });
  }

  pub fn update_writers_reader_proxy(&mut self, writer: &GUID, reader: RtpsReaderProxy) -> bool {
    let mut proxies = match self.writers_reader_proxies.get(&writer) {
      Some(prox) => prox.clone(),
      None => Vec::new(),
    };

    let value: Option<usize> = proxies
      .iter()
      .position(|x| x.remote_reader_guid == reader.remote_reader_guid);
    match value {
      Some(val) => proxies[val] = reader,
      None => proxies.push(reader),
    };

    self.writers_reader_proxies.insert(writer.clone(), proxies);
    true
  }
}

#[cfg(test)]

mod tests {
  use super::*;

  use crate::test::test_data::{
    subscription_builtin_topic_data, spdp_participant_data, reader_proxy_data,
  };

  use crate::structure::guid::*;

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
    let reader = RtpsReaderProxy::new(GUID::new());
    discoverydb.update_writers_reader_proxy(&GUID::new(), reader);
    assert_eq!(discoverydb.writers_reader_proxies.len(), 1);

    // TODO: more tests :)
  }

  #[test]
  fn discdb_subscription_operations() {
    let mut discovery_db = DiscoveryDB::new();

    let topic_name = "Foobar".to_string();
    let topic_name2 = "Barfoo".to_string();

    let writer = RtpsWriterProxy::new(GUID::new(), Vec::new(), Vec::new(), EntityId::createCustomEntityID([1, 2, 3], 111) );
    let writer_key = writer.remote_writer_guid.clone();
    discovery_db.add_local_topic_writer(topic_name.clone(), writer);
    assert_eq!(discovery_db.local_topic_writers.len(), 1);

    let writer2 = RtpsWriterProxy::new(GUID::new(), Vec::new(), Vec::new(), EntityId::createCustomEntityID([1, 2, 4], 111));
    let _writer2_key = writer2.remote_writer_guid.clone();
    discovery_db.add_local_topic_writer(topic_name2.clone(), writer2);
    assert_eq!(discovery_db.local_topic_writers.len(), 2);

    // creating data
    let reader1 = reader_proxy_data().unwrap();
    let mut reader1sub = subscription_builtin_topic_data().unwrap();
    reader1sub.key = reader1.remote_reader_guid.clone();
    reader1sub.topic_name = Some(topic_name.clone());
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
    reader2sub.topic_name = Some(topic_name2.clone());
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
}
