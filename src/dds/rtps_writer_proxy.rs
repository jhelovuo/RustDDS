use crate::structure::locator::LocatorList;
use crate::structure::guid::{EntityId, GUID};
use crate::{
  discovery::data_types::topic_data::DiscoveredWriterData,
  structure::sequence_number::{SequenceNumber},
  structure::time::Timestamp,
};
use std::collections::BTreeMap;
//use std::time::Instant;

#[derive(Debug,Clone)]
pub(crate) struct RtpsWriterProxy {
  /// Identifies the remote matched Writer
  pub remote_writer_guid: GUID,

  /// List of unicast (address, port) combinations that can be used to send
  /// messages to the matched Writer or Writers. The list may be empty.
  pub unicast_locator_list: LocatorList,

  /// List of multicast (address, port) combinations that can be used to send
  /// messages to the matched Writer or Writers. The list may be empty.
  pub multicast_locator_list: LocatorList,

  /// Identifies the group to which the matched Reader belongs
  pub remote_group_entity_id: EntityId,

  /// List of sequence_numbers received from the matched RTPS Writer
  // TODO: When should they be removed from here?
  // Or keep separately track of latest timestamp.
  changes: BTreeMap<SequenceNumber, Timestamp>,

  pub received_heartbeat_count: i32,

  pub sent_ack_nack_count: i32,
  
  //pub qos : QosPolicies,
}

impl RtpsWriterProxy {
  pub fn new(
    remote_writer_guid: GUID,
    unicast_locator_list: LocatorList,
    multicast_locator_list: LocatorList,
    remote_group_entity_id: EntityId,
  ) -> Self {
    Self {
      remote_writer_guid,
      unicast_locator_list,
      multicast_locator_list,
      remote_group_entity_id,
      changes: BTreeMap::new(),
      received_heartbeat_count: 0,
      sent_ack_nack_count: 0,
    }
  }

  pub fn update_contents(&mut self, other: RtpsWriterProxy) {
    self.unicast_locator_list = other.unicast_locator_list;
    self.multicast_locator_list = other.multicast_locator_list;
    self.remote_group_entity_id = other.remote_group_entity_id;
  }

  // TODO: This is quite inefficient
  pub fn last_change_timestamp(&self) -> Option<Timestamp> {
    self.changes.values().max().copied()
  }

  pub fn no_changes(&self) -> bool {
    self.changes.is_empty()
  }

  pub fn get_missing_sequence_numbers(
    &self,
    hb_first_sn: SequenceNumber,
    hb_last_sn: SequenceNumber,
  ) -> Vec<SequenceNumber> {
    let mut missing_seqnums = Vec::with_capacity(32); // out of hat value

    for msq in SequenceNumber::range_inclusive(hb_first_sn,hb_last_sn)  {
      if !self.changes.contains_key(&msq) && msq >= SequenceNumber::default() {
        missing_seqnums.push(msq)
      }
    }

    missing_seqnums
  }

  pub fn changes_are_missing(
    &self,
    hb_first_sn: SequenceNumber,
    hb_last_sn: SequenceNumber,
  ) -> bool {
    if hb_last_sn < hb_first_sn { // This means writer has nothing to send
      return false
    }
    for s in SequenceNumber::range_inclusive(hb_first_sn,hb_last_sn)  {
      if !self.changes.contains_key(&s) { return true }
    }
    false
      
    // let range_length = i64::from(hb_last_sn.sub(hb_first_sn)) as usize + 1;
    // let seq_count = self
    //   .changes
    //   .iter()
    //   .filter(|(&sq, _)| sq >= hb_first_sn && sq <= hb_last_sn)
    //   .count();

    // seq_count < range_length
  }

  pub fn contains_change(&self, seqnum: SequenceNumber) -> bool {
    self.changes.contains_key(&seqnum)
  }

  pub fn received_changes_add(&mut self, seq_num: SequenceNumber, instant: Timestamp) {
    self.changes.insert(seq_num, instant);
  }

  pub fn available_changes_max(&self) -> Option<SequenceNumber> {
    // TODO: replace this when BTreeMap function last_key_value() is in stable release
    self.changes.keys().next_back().copied()
  }

  pub fn available_changes_min(&self) -> Option<SequenceNumber> {
    self.changes.keys().next().copied()
    // TODO: replace this when BTreeMap function first_key_value() is in stable release
  }

  pub fn set_irrelevant_change(&mut self, seq_num: SequenceNumber) -> Option<Timestamp> {
    self.changes.remove(&seq_num)
  }

  pub fn irrelevant_changes_up_to(&mut self, smallest_seqnum: SequenceNumber) -> Vec<Timestamp> {
    let mut remove = Vec::new();
    for (&seqnum, _) in self.changes.iter() {
      if seqnum < smallest_seqnum {
        remove.push(seqnum);
      }
    }

    let mut instants = Vec::new();
    for &rm in remove.iter() {
      match self.changes.remove(&rm) {
        Some(i) => instants.push(i),
        None => (),
      };
    }

    instants
  }

  pub fn from_discovered_writer_data(discovered_writer_data: &DiscoveredWriterData) 
    -> RtpsWriterProxy 
  {
    RtpsWriterProxy {
      remote_writer_guid: discovered_writer_data.writer_proxy.remote_writer_guid.clone(),
      remote_group_entity_id: EntityId::ENTITYID_UNKNOWN,
      unicast_locator_list: discovered_writer_data
        .writer_proxy
        .unicast_locator_list
        .clone(),
      multicast_locator_list: discovered_writer_data
        .writer_proxy
        .multicast_locator_list
        .clone(),
      changes: BTreeMap::new(),
      received_heartbeat_count: 0,
      sent_ack_nack_count: 0,
    }
  }
}
