use crate::structure::locator::LocatorList;
use crate::structure::guid::{EntityId, GUID};
use crate::structure::sequence_number::{SequenceNumber};
use std::collections::HashMap;
use std::time::Instant;

pub struct RtpsWriterProxy {
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
  pub changes: HashMap<SequenceNumber, Instant>,

  pub received_heartbeat_count: i32,
}

impl RtpsWriterProxy {
  pub fn new(
    remote_writer_guid: GUID,
    unicast_locator_list: LocatorList,
    multicast_locator_list: LocatorList,
  ) -> Self {
    Self {
      remote_writer_guid,
      unicast_locator_list,
      multicast_locator_list,
      remote_group_entity_id: EntityId::ENTITYID_UNKNOWN,
      changes: HashMap::new(),
      received_heartbeat_count: 0,
    }
  }

  pub fn received_changes_add(&mut self, seq_num: SequenceNumber, instant: Instant) {
    self.changes.insert(seq_num, instant);
  }

  pub fn available_changes_max(&self) -> Option<&SequenceNumber> {
    if let Some((seqnum, _)) = self.changes.iter().max() {
      return Some(seqnum);
    }
    None
  }

  pub fn available_changes_min(&self) -> Option<&SequenceNumber> {
    if let Some((seqnum, _)) = self.changes.iter().min() {
      return Some(seqnum);
    }
    None
  }

  pub fn irrelevant_changes_set(&mut self, seq_num: SequenceNumber) -> Instant {
    self.changes.remove(&seq_num).unwrap()
  }

  pub fn irrelevant_changes_up_to(&mut self, smallest_seqnum: SequenceNumber) -> Vec<Instant> {
    let mut removed = Vec::new();
    for (seqnum, _) in self.changes.clone().iter() {
      if seqnum < &smallest_seqnum {
        removed.push(self.changes.remove(seqnum).unwrap());
      }
    }
    removed
  }
}
