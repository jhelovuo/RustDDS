use core::ops::Bound::{Excluded, Unbounded};
use std::{cmp::max, collections::BTreeMap};

use enumflags2::BitFlags;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::{
  dds::{ddsdata::DDSData, fragment_assembler::FragmentAssembler},
  discovery::data_types::topic_data::DiscoveredWriterData,
  messages::submessages::submessages::{DATAFRAG_Flags, DataFrag},
  structure::{
    guid::{EntityId, GUID},
    locator::LocatorList,
    sequence_number::SequenceNumber,
    time::Timestamp,
  },
};

#[derive(Debug)] // these are not cloneable, because contined data may be large
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

  ack_base: SequenceNumber, // We can ACK everything before this number.
  // ack_base can be increased from N-1 to N, if we receive DATA with SequenceNumber N-1
  // heartbeat(first,last) => ack_base can be increased to first.
  // GAP is treated like receiving a message.

  //pub qos : QosPolicies,
  fragment_assembler: Option<FragmentAssembler>,
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
      fragment_assembler: None,
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

  pub fn last_change_timestamp(&self) -> Option<Timestamp> {
    self.changes.values().next_back().copied()
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

    let mut we_have = self
      .changes
      .range(SequenceNumber::range_inclusive(hb_first_sn, hb_last_sn))
      .map(|e| *e.0);
    let mut have_head = we_have.next();

    for s in SequenceNumber::range_inclusive(hb_first_sn, hb_last_sn) {
      match have_head {
        None => missing_seqnums.push(s),
        Some(have_sn) => {
          if have_sn == s {
            have_head = we_have.next();
          } else {
            missing_seqnums.push(s);
          }
        }
      }
    }
    missing_seqnums
  }

  pub fn changes_are_missing(
    &self,
    hb_first_sn: SequenceNumber,
    hb_last_sn: SequenceNumber,
  ) -> bool {
    if hb_last_sn < hb_first_sn {
      // This means writer has nothing to send
      return false;
    }

    let we_have = self
      .changes
      .range(SequenceNumber::range_inclusive(hb_first_sn, hb_last_sn))
      .map(|e| *e.0);
    SequenceNumber::range_inclusive(hb_first_sn, hb_last_sn).ne(we_have)
  }

  pub fn contains_change(&self, seqnum: SequenceNumber) -> bool {
    self.changes.contains_key(&seqnum)
  }

  pub fn received_changes_add(&mut self, seq_num: SequenceNumber, instant: Timestamp) {
    self.changes.insert(seq_num, instant);

    // We get to advance ack_base if it was equal to seq_num
    // If ack_base < seq_num, we are still missing seq_num-1 or others below
    // If ack_base > seq_num, this is either a duplicate or ack_base was wrong.
    // Remember, ack_base is the SN one past the last received/irrelevant SN.
    if seq_num == self.ack_base {
      let mut s = seq_num;
      for (&sn, _) in self.changes.range((Excluded(&seq_num), Unbounded)) {
        if sn == s + SequenceNumber::new(1) {
          // got consecutive number from previous
          s = s + SequenceNumber::new(1) // and continue looping
        } else {
          break; // not consecutive
        }
      } // end for
        // Now we have received everything up to and including s. Ack base is one up
        // from that.
      self.ack_base = s + SequenceNumber::new(1)
    }
  }

  pub fn available_changes_max(&self) -> Option<SequenceNumber> {
    // TODO: replace this when BTreeMap function last_key_value() is in stable
    // release
    self.changes.keys().next_back().copied()
  }

  pub fn available_changes_min(&self) -> Option<SequenceNumber> {
    self.changes.keys().next().copied()
    // TODO: replace this when BTreeMap function first_key_value() is in stable
    // release
  }

  pub fn set_irrelevant_change(&mut self, seq_num: SequenceNumber) -> Option<Timestamp> {
    self.changes.remove(&seq_num)
    // TODO: Update ack_base
  }

  pub fn irrelevant_changes_range(
    &mut self,
    remove_from: SequenceNumber,
    remove_until_before: SequenceNumber,
  ) -> BTreeMap<SequenceNumber, Timestamp> {
    let mut removed_and_after = self.changes.split_off(&remove_from);
    let mut after = removed_and_after.split_off(&remove_until_before);
    let removed = removed_and_after;
    self.changes.append(&mut after);

    if self.ack_base >= remove_from {
      self.ack_base = max(remove_until_before, self.ack_base)
    }

    removed
  }

  // smallest_seqnum is the lowest key to be retained
  pub fn irrelevant_changes_up_to(
    &mut self,
    smallest_seqnum: SequenceNumber,
  ) -> BTreeMap<SequenceNumber, Timestamp> {
    // split_off() Splits the collection into two at the given key.
    // Returns everything after the given key, including the key.
    let remaining_changes = self.changes.split_off(&smallest_seqnum);
    let irrelevant = std::mem::replace(&mut self.changes, remaining_changes);

    self.ack_base = max(smallest_seqnum, self.ack_base);

    irrelevant
  }

  pub fn from_discovered_writer_data(
    discovered_writer_data: &DiscoveredWriterData,
  ) -> RtpsWriterProxy {
    RtpsWriterProxy {
      remote_writer_guid: discovered_writer_data.writer_proxy.remote_writer_guid,
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
      fragment_assembler: None,
    }
  } // fn

  pub fn handle_datafrag(
    &mut self,
    datafrag: DataFrag,
    flags: BitFlags<DATAFRAG_Flags>,
  ) -> Option<DDSData> {
    match self.fragment_assembler {
      Some(ref mut fa) => fa.new_datafrag(datafrag, flags),
      None => {
        let mut fa = FragmentAssembler::new(datafrag.fragment_size);
        //TODO: Test that the fragment size is not zero
        let ret = fa.new_datafrag(datafrag, flags);
        self.fragment_assembler = Some(fa);
        ret
      }
    }
  } // fn
} // impl
