use crate::structure::{
  locator::{Locator, LocatorList},
  guid::{EntityId, GUID},
  sequence_number::{SequenceNumber},
};
use crate::{
  common::{bit_set::BitSetRef},
  discovery::data_types::topic_data::DiscoveredReaderData,
};
use std::{
  collections::HashSet,
  net::{SocketAddr, Ipv4Addr},
};

#[derive(Debug, PartialEq, Clone)]
///ReaderProxy class represents the information an RTPS StatefulWriter maintains on each matched RTPS Reader
pub struct RtpsReaderProxy {
  ///Identifies the remote matched RTPS Reader that is represented by the ReaderProxy
  pub remote_reader_guid: GUID,
  /// Identifies the group to which the matched Reader belongs
  pub remote_group_entity_id: EntityId,
  /// List of unicast locators (transport, address, port combinations) that can be used to send messages to the matched RTPS Reader. The list may be empty
  pub unicast_locator_list: LocatorList,
  /// List of multicast locators (transport, address, port combinations) that can be used to send messages to the matched RTPS Reader. The list may be empty
  pub multicast_locator_list: LocatorList,
  /// List of CacheChange changes as they relate to the matched RTPS Reader.
  //changes_for_reader :  Arc<HistoryCache>,

  /// Specifies whether the remote matched RTPS Reader expects in-line QoS to be sent along with any data.
  pub expects_in_line_qos: bool,
  /// Specifies whether the remote Reader is responsive to the Writer
  pub is_active: bool,

  // this list keeps sequence numbers from reader negative acknack messages
  requested_changes: HashSet<SequenceNumber>,

  // this keeps sequence number of reader recieved (acknack recieved) messages
  largest_acked_change: Option<SequenceNumber>,

  unsent_changes: HashSet<SequenceNumber>,
}

impl RtpsReaderProxy {
  pub fn new(remote_reader_guid: GUID) -> RtpsReaderProxy {
    RtpsReaderProxy {
      remote_reader_guid,
      remote_group_entity_id: EntityId::ENTITYID_UNKNOWN,
      unicast_locator_list: LocatorList::new(),
      multicast_locator_list: LocatorList::new(),
      //changes_for_reader : writer.history_cache.clone(),
      expects_in_line_qos: false,
      is_active: true,
      requested_changes: HashSet::new(),
      unsent_changes: HashSet::new(),
      largest_acked_change: None,
    }
  }

  pub fn from_discovered_reader_data(
    discovered_reader_data: &DiscoveredReaderData,
  ) -> Option<RtpsReaderProxy> {
    let remote_reader_guid = match &discovered_reader_data.reader_proxy.remote_reader_guid {
      Some(v) => v,
      None => {
        println!("Failed to convert DiscoveredReaderData to RtpsReaderProxy. No GUID");
        return None;
      }
    };

    let expects_inline_qos = match &discovered_reader_data.reader_proxy.expects_inline_qos {
      Some(v) => v.clone(),
      None => false,
    };

    Some(RtpsReaderProxy {
      remote_reader_guid: remote_reader_guid.clone(),
      remote_group_entity_id: EntityId::ENTITYID_UNKNOWN,
      unicast_locator_list: discovered_reader_data
        .reader_proxy
        .unicast_locator_list
        .clone(),
      multicast_locator_list: discovered_reader_data
        .reader_proxy
        .multicast_locator_list
        .clone(),
      expects_in_line_qos: expects_inline_qos,
      is_active: true,
      requested_changes: HashSet::new(),
      unsent_changes: HashSet::new(),
      largest_acked_change: None,
    })
  }

  pub fn update(&mut self, updated: &RtpsReaderProxy) {
    if self.remote_reader_guid == updated.remote_reader_guid {
      self.unicast_locator_list = updated.unicast_locator_list.clone();
      self.multicast_locator_list = updated.multicast_locator_list.clone();
      self.expects_in_line_qos = updated.expects_in_line_qos.clone();
    }
  }

  pub fn new_for_unit_testing(port_number: u16) -> RtpsReaderProxy {
    let mut unicastLocators = LocatorList::new();
    let locator = Locator::from(SocketAddr::new(
      std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
      port_number,
    ));
    unicastLocators.push(locator);
    RtpsReaderProxy {
      remote_reader_guid: GUID::new(),
      remote_group_entity_id: EntityId::ENTITYID_UNKNOWN,
      unicast_locator_list: unicastLocators,
      multicast_locator_list: LocatorList::new(),
      //changes_for_reader : writer.history_cache.clone(),
      expects_in_line_qos: false,

      is_active: true,
      requested_changes: HashSet::new(),
      unsent_changes: HashSet::new(),
      largest_acked_change: None,
    }
  }

  pub fn can_send(&self) -> bool {
    if self.can_send_unsend() || self.can_send_requested() {
      return true;
    } else {
      return false;
    }
  }

  fn can_send_unsend(&self) -> bool {
    if self.unsent_changes().len() > 0 {
      return true;
    }
    return false;
  }

  fn can_send_requested(&self) -> bool {
    if self.requested_changes().len() > 0 {
      return true;
    }
    return false;
  }

  /// returns list of sequence numbers that are requested by reader with acknack
  pub fn requested_changes(&self) -> &HashSet<SequenceNumber> {
    return &self.requested_changes;
  }

  pub fn unsent_changes(&self) -> &HashSet<SequenceNumber> {
    return &self.unsent_changes;
  }

  pub fn next_requested_change(&self) -> Option<&SequenceNumber> {
    let mut min_value = SequenceNumber::from(std::i64::MAX);
    let mut min: Option<&SequenceNumber> = None;
    for request in self.requested_changes() {
      if request < &min_value {
        min = Some(request);
        min_value = *request;
      }
    }
    return min;
  }

  pub fn next_unsent_change(&self) -> Option<SequenceNumber> {
    let mut min_value = SequenceNumber::from(std::i64::MAX);
    let mut min: Option<SequenceNumber> = None;
    for &request in self.unsent_changes() {
      if request > SequenceNumber::from(0) && request < min_value {
        min = Some(request);
        min_value = request;
      }
    }
    return min;
  }

  pub fn add_requested_changes(&mut self, sequence_numbers: BitSetRef) {
    for number in sequence_numbers.iter() {
      self
        .requested_changes
        .insert(SequenceNumber::from(number as i64));
    }
  }

  /// this should be called everytime a new CacheChange is set to RTPS writer HistoryCache
  pub fn unsend_changes_set(&mut self, sequence_number: SequenceNumber) {
    if sequence_number > SequenceNumber::from(0) {
      self.unsent_changes.insert(sequence_number);
    }
  }

  /// this should be called everytime next_unsent_change is called and change is sent
  pub fn remove_unsend_change(&mut self, sequence_number: SequenceNumber) {
    self.unsent_changes.remove(&sequence_number);
  }
  ///This operation changes the ChangeForReader status of a set of changes for the reader represented by
  ///ReaderProxy ‘the_reader_proxy.’ The set of changes with sequence number smaller than or equal to the value
  ///‘committed_seq_num’ have their status changed to ACKNOWLEDGED
  pub fn acked_changes_set(&mut self, sequence_number: SequenceNumber) {
    self.largest_acked_change = Some(sequence_number);
  }

  pub fn sequence_is_acked(&self, sequence_number: &SequenceNumber) -> bool {
    if self.largest_acked_change.is_none() {
      return false;
    }
    if &self.largest_acked_change.unwrap() >= sequence_number {
      return true;
    }
    return false;
  }
}

pub enum ChangeForReaderStatusKind {
  UNSENT,
  NACKNOWLEDGED,
  REQUESTED,
  ACKNOWLEDGED,
  UNDERWAY,
}

///The RTPS ChangeForReader is an association class that maintains information of a CacheChange in the RTPS
///Writer HistoryCache as it pertains to the RTPS Reader represented by the ReaderProxy
pub struct RTPSChangeForReader {
  ///Indicates the status of a CacheChange relative to the RTPS Reader represented by the ReaderProxy.
  pub kind: ChangeForReaderStatusKind,
  ///Indicates whether the change is relevant to the RTPS Reader represented by the ReaderProxy.
  pub is_relevant: bool,
}

impl RTPSChangeForReader {
  pub fn new() -> RTPSChangeForReader {
    RTPSChangeForReader {
      kind: ChangeForReaderStatusKind::UNSENT,
      is_relevant: true,
    }
  }
}
