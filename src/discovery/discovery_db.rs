use std::{time::Duration, collections::HashMap};

use crate::structure::{guid::GUID, duration::Duration as SDuration};

use crate::dds::rtps_reader_proxy::RtpsReaderProxy;
use crate::dds::rtps_writer_proxy::RtpsWriterProxy;
use super::data_types::spdp_participant_data::SPDPDiscoveredParticipantData;

pub struct DiscoveryDB {
  participant_proxies: HashMap<GUID, SPDPDiscoveredParticipantData>,
  writers_reader_proxies: HashMap<GUID, Vec<RtpsReaderProxy>>,
  readers_writer_proxies: HashMap<GUID, Vec<RtpsWriterProxy>>,
}

impl DiscoveryDB {
  pub fn new() -> DiscoveryDB {
    DiscoveryDB {
      participant_proxies: HashMap::new(),
      writers_reader_proxies: HashMap::new(),
      readers_writer_proxies: HashMap::new(),
    }
  }

  pub fn update_participant(&mut self, data: &SPDPDiscoveredParticipantData) {
    match data.participant_guid {
      Some(guid) => {
        self.participant_proxies.insert(guid, data.clone());
      }
      _ => (),
    };
  }

  pub fn participant_cleanup(&mut self) {
    let instant = time::precise_time_ns();
    self.participant_proxies.retain(|_, d| {
      // TODO: do we always assume that lease duration exists?
      SDuration::from(Duration::from_nanos(instant - d.updated_time))
        < *d.lease_duration.as_ref().unwrap()
    });
  }

  pub fn add_writers_reader_proxy(&mut self, writer: GUID, reader: RtpsReaderProxy) {
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

    self.writers_reader_proxies.insert(writer, proxies);
  }
}

#[cfg(test)]

mod tests {
  use super::*;

  use crate::test::test_data::spdp_participant_data;

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
    discoverydb.add_writers_reader_proxy(GUID::new(), reader);
    assert_eq!(discoverydb.writers_reader_proxies.len(), 1);

    // TODO: more tests :)
  }
}
