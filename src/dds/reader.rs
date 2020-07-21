use crate::structure::entity::Entity;
use crate::structure::endpoint::{Endpoint, EndpointAttributes};
use crate::structure::history_cache::HistoryCache;
use crate::messages::submessages::data::Data;
use crate::messages::submessages::ack_nack::AckNack;
use crate::messages::submessages::heartbeat::Heartbeat;
use crate::messages::submessages::gap::Gap;
use crate::structure::entity::EntityAttributes;
use crate::structure::guid::GUID;
use crate::structure::sequence_number::{SequenceNumber, SequenceNumberSet};
use mio::event::Evented;
use mio::{Ready, Registration, Poll, PollOpt, Token, SetReadiness};

use std::io;
use std::collections::HashSet;

use std::time::Duration;
use crate::structure::cache_change::CacheChange;

#[derive(Debug)]
pub struct Reader {
  set_readiness: SetReadiness,
  registration: Registration,

  history_cache: HistoryCache, // atm done with the assumption, that only one writers cache is monitored
  entity_attributes: EntityAttributes,
  pub enpoint_attributes: EndpointAttributes,

  heartbeat_response_delay: Duration,
  heartbeat_supression_duration: Duration,

  sent_ack_nack_count: i32,
  received_hearbeat_count: i32,
} // placeholder

impl Reader {
  pub fn new(
    guid: GUID,
    set_readiness: SetReadiness,
    registration: Registration,
  ) -> Reader {
    Reader {
      set_readiness,
      registration,
      history_cache: HistoryCache::new(),
      entity_attributes: EntityAttributes{guid},
      enpoint_attributes: EndpointAttributes::default(),

      heartbeat_response_delay: Duration::new(0, 500_000_000), // 0,5sec
      heartbeat_supression_duration: Duration::new(0, 0),
      sent_ack_nack_count: 0,
      received_hearbeat_count: 0,
    }
  }
  // TODO: check if it's necessary to implement different handlers for discovery
  // and user messages

  // handles regular data message and updates history cache
  pub fn handle_data_msg(&mut self, data: Data) { 
    let user_data = true; // Different action for discovery data?
    if user_data {
      // TODO! Sequence number check?
      self.make_cache_change(data);
    }else {
      // is discovery data
      todo!();
    }
    self.notify_cache_change();
  }

  // send ack_nack response if necessary. spec page 104
  pub fn handle_heartbeat_msg(
    &mut self, heartbeat: Heartbeat, 
    final_flag_set: bool
  ) -> bool {

    if heartbeat.count <= self.received_hearbeat_count { 
      // Already received newer or same
      return false;
    }
    self.received_hearbeat_count = heartbeat.count;
    
    // remove fragmented changes untill first_sn ???
    //self.history_cache.remove_changes_up_to(heartbeat.first_sn);

    let last_seq_num: SequenceNumber;
    if let Some(num) = self.history_cache.get_seq_num_max() {
      last_seq_num = *num;
    } else {
      last_seq_num = SequenceNumber::from(0);
    }

    // See if ack_nack is needed.
    let changes_missing = last_seq_num < heartbeat.last_sn;
    let need_ack_nack = changes_missing || !final_flag_set;

    if need_ack_nack {
      let mut reader_sn_state = 
       SequenceNumberSet::new(last_seq_num);
      for seq_num in i64::from(last_seq_num)..i64::from(heartbeat.last_sn) {
        reader_sn_state.insert(SequenceNumber::from(seq_num));
      }

      let response_ack_nack = AckNack{
        reader_id: self.get_entity_id(),
        writer_id: heartbeat.writer_id,
        reader_sn_state,
        count: self.sent_ack_nack_count,
      };// Send this AckNack!!!!!!!!!
      self.sent_ack_nack_count += 1;

    }
    self.notify_cache_change();
    need_ack_nack
  }

  pub fn handle_gap_msg(&mut self, gap: Gap){
    let mut irrelevant_changes_set = HashSet::new();
    // Irrelevant sequence numbers communicated in the Gap message are 
    // composed of two groups
    // 1. All sequence numbers in the range gapStart <= sequence_number <= gapList.base -1
    for seqnum_i64 in i64::from(gap.gap_start)..i64::from(gap.gap_list.base) {
      irrelevant_changes_set.insert(SequenceNumber::from(seqnum_i64));
    }
    // 2. All the sequence numbers that appear explicitly listed in the gapList.
    for seqnum in &mut gap.gap_list.set.into_iter() {
      irrelevant_changes_set.insert(SequenceNumber::from(seqnum as i64));
    }

    //Remove irrelevant?
    for seqnum in irrelevant_changes_set {
      self.history_cache.remove_change(seqnum);
    }

    self.notify_cache_change();
  }

  // update history cache
  fn make_cache_change(&mut self, data: Data) {
    let change =  CacheChange::new(
      self.get_guid(),
      data.writer_sn,
      Some(data),
    );
    self.history_cache.add_change(change);
  }

  // notifies DataReaders (or any listeners that history cache has changed for this reader)
  // likely use of mio channel
  fn notify_cache_change(&self) {
    println!("Trying to send a notification of cache change...");
    self.set_readiness.set_readiness(Ready::readable()).unwrap()
  }
}

impl Entity for Reader {
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
  }
}
  
impl Endpoint for Reader {
  fn as_endpoint(&self) -> &crate::structure::endpoint::EndpointAttributes {
    &self.enpoint_attributes
  }
}

impl PartialEq for Reader {
    // Ignores registration
    fn eq(&self, other: &Self) -> bool {
        self.history_cache == other.history_cache &&
        self.entity_attributes == other.entity_attributes &&
        self.enpoint_attributes == other.enpoint_attributes &&
        self.heartbeat_response_delay == other.heartbeat_response_delay &&
        self.heartbeat_supression_duration == other.heartbeat_supression_duration &&
        self.sent_ack_nack_count == other.sent_ack_nack_count &&
        self.received_hearbeat_count == other.received_hearbeat_count
    }

}

impl Evented for Reader {
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.registration.register(poll, token, interest, opts)
  }
  fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.registration.reregister(poll, token, interest, opts)
  }
  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.registration.deregister(poll)
  }
}


#[cfg(test)]
mod tests {
  use super::*;
  use crate::structure::guid::{GUID, EntityId};

  #[test]
  fn rtpsreader_handle_data() {


    let new_guid = GUID::new();
    let (
      register_reader, 
      set_readiness_of_reader) = Registration::new2();

    let mut new_reader = Reader::new(
      new_guid, 
      set_readiness_of_reader, 
      register_reader);

    let d = Data::default();
    let d_seqnum = d.writer_sn;
    new_reader.handle_data_msg(d.clone()); 

    let change =  CacheChange::new(
      new_reader.get_guid(),
      d_seqnum,
      Some(d),
    );

    let res = 
      new_reader.history_cache.get_change(d_seqnum);
    assert_eq!(*res.unwrap(),change);
  }

  #[test]
  fn rtpsreader_handle_heartbeat() {
    let new_guid = GUID::new();
    let (
      register_reader, 
      set_readiness_of_reader) = Registration::new2();

    let mut new_reader = Reader::new(
      new_guid, 
      set_readiness_of_reader, 
      register_reader);

    let writer_id = EntityId::default();
    let d = Data::default();
    let mut changes = Vec::new();


    let hb_new = Heartbeat{
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // First hearbeat from a new writer
      last_sn: SequenceNumber::from(0),
      count: 1,
    };
    assert!(!new_reader.handle_heartbeat_msg(hb_new, true)); // should be false, no ack


    let hb_one = Heartbeat{
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // Only one in writers cache
      last_sn: SequenceNumber::from(1),
      count: 2,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_one, false)); // Should send an ack_nack

    // After ack_nack, will receive the following change
    let change =  CacheChange::new(
      new_reader.get_guid(),
      SequenceNumber::from(1),
      Some(d.clone()),
    );
    new_reader.history_cache.add_change(change.clone());
    changes.push(change);


    // Duplicate
    let hb_one2 = Heartbeat{
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // Only one in writers cache
      last_sn: SequenceNumber::from(1),
      count: 2,
    };
    assert!(!new_reader.handle_heartbeat_msg(hb_one2, false));


    let hb_3_1 = Heartbeat{
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // writer has last 2 in cache
      last_sn: SequenceNumber::from(3), // writer has written 3 samples
      count: 3,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_3_1, false)); // Should send an ack_nack

    // After ack_nack, will receive the following changes
    let change =  CacheChange::new(
      new_reader.get_guid(),
      SequenceNumber::from(2),
      Some(d.clone()),
    );
    new_reader.history_cache.add_change(change.clone());
    changes.push(change);
    let change =  CacheChange::new(
      new_reader.get_guid(),
      SequenceNumber::from(3),
      Some(d),
    );
    new_reader.history_cache.add_change(change.clone());
    changes.push(change);

    let hb_none = Heartbeat{
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(4), // writer has no samples available
      last_sn: SequenceNumber::from(3), // writer has written 3 samples
      count: 4,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_none, false)); 

    assert_eq!(new_reader.sent_ack_nack_count, 3);
  }

  #[test]
  fn rtpsreader_handle_gap() {
    let new_guid = GUID::new();
    let (
      register_reader, 
      set_readiness_of_reader) = Registration::new2();

    let mut reader = Reader::new(
      new_guid, 
      set_readiness_of_reader, 
      register_reader);

    let n: i64 = 10;
    let d = Data::default();
    let mut changes = Vec::new();

    for i in 0..n {
      let change =  CacheChange::new(
        reader.get_guid(),
        SequenceNumber::from(i),
        Some(d.clone()),
      );
      reader.history_cache.add_change(change.clone());
      changes.push(change);
    }

    let writer_id = EntityId::default();

    // make sequence numbers 1-3 and 5 7 irrelevant 
    let mut gap_list = 
     SequenceNumberSet::new(SequenceNumber::from(4));
    gap_list.insert(SequenceNumber::from(5+4)); // TODO! Why do you need to add base!?
    gap_list.insert(SequenceNumber::from(7+4)); 

    let gap = Gap{
      reader_id: reader.get_entity_id(),
      writer_id,
      gap_start: SequenceNumber::from(1),
      gap_list,
    };

    reader.handle_gap_msg(gap);

    assert_eq!(reader.history_cache.get_change(SequenceNumber::from(0)), Some(&changes[0]));
    assert_eq!(reader.history_cache.get_change(SequenceNumber::from(1)), None);
    assert_eq!(reader.history_cache.get_change(SequenceNumber::from(2)), None);
    assert_eq!(reader.history_cache.get_change(SequenceNumber::from(3)), None);
    assert_eq!(reader.history_cache.get_change(SequenceNumber::from(4)), Some(&changes[4]));
    assert_eq!(reader.history_cache.get_change(SequenceNumber::from(5)), None);
    assert_eq!(reader.history_cache.get_change(SequenceNumber::from(6)), Some(&changes[6]));
    assert_eq!(reader.history_cache.get_change(SequenceNumber::from(7)), None);
    assert_eq!(reader.history_cache.get_change(SequenceNumber::from(8)), Some(&changes[8]));
    assert_eq!(reader.history_cache.get_change(SequenceNumber::from(9)), Some(&changes[9]));
  }
 

}
