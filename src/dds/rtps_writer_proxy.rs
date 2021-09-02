use core::ops::Bound::Excluded;
use core::ops::Bound::Unbounded;
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
  changes: BTreeMap<SequenceNumber, Timestamp>,
  // The changes map is cleaned on heartbeat messages. The changes no longer available are dropped.

  pub received_heartbeat_count: i32,

  pub sent_ack_nack_count: i32,

  ack_base : SequenceNumber, // We can ACK everything before this number.
  // ack_base can be increased from N-1 to N, ifwe receive SequenceNumber N-1
  // heartbeat(first,last) => ack_base can be increased to first
  // gap is treated like receiving message
  
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
      ack_base: SequenceNumber::default(),
    }
  }

  pub fn all_ackable_before(&self) -> SequenceNumber {
    self.ack_base
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
  }

  pub fn contains_change(&self, seqnum: SequenceNumber) -> bool {
    self.changes.contains_key(&seqnum)
  }

  pub fn received_changes_add(&mut self, seq_num: SequenceNumber, instant: Timestamp) {
    self.changes.insert(seq_num, instant);

    // We get to advance ack_base if it was equal to seq_num
    // If ack_base < seq_num, we are still missing seq_num-1 or others below
    // If ack_base > seq_num, this is either a duplicate or ack_base was wrong.
    if seq_num == self.ack_base {
      let mut s = seq_num;
      for (&sn,_) in self.changes.range((Excluded(&seq_num), Unbounded)) {
        // TODO: Implement this.
      }
    }
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
    // TODO: Update ack_base
  }

  pub fn irrelevant_changes_range(&mut self, 
    remove_from: SequenceNumber, 
    remove_until_before: SequenceNumber ) -> BTreeMap<SequenceNumber, Timestamp> 
  {
    let mut removed_and_after = self.changes.split_off(&remove_from);
    let mut after = removed_and_after.split_off(&remove_until_before);
    let removed = removed_and_after;
    self.changes.append(&mut after);
    // TODO: Update ack_base
    removed
  }

  // smallest_seqnum is the lowest key to be retained
  pub fn irrelevant_changes_up_to(&mut self, smallest_seqnum: SequenceNumber) 
    -> BTreeMap<SequenceNumber, Timestamp> 
  {
    // split_off() Splits the collection into two at the given key. 
    // Returns everything after the given key, including the key.
    let remaining_changes = self.changes.split_off(&smallest_seqnum);
    std::mem::replace(&mut self.changes, remaining_changes) // returns the irrelevant changes
    // TODO: Update ack_base
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
      ack_base: SequenceNumber::default(),
    }
  }
}
