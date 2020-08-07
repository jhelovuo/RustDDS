use crate::structure::entity::Entity;
use crate::structure::endpoint::{Endpoint, EndpointAttributes};
use crate::structure::history_cache::HistoryCache;
use crate::messages::submessages::data::Data;

use crate::dds::ddsdata::DDSData;
use crate::dds::rtps_writer_proxy::RtpsWriterProxy;
use crate::messages::submessages::ack_nack::AckNack;
use crate::messages::submessages::heartbeat::Heartbeat;
use crate::messages::submessages::gap::Gap;
use crate::structure::entity::EntityAttributes;
use crate::structure::guid::{GuidPrefix, GUID};
use crate::structure::sequence_number::{SequenceNumber, SequenceNumberSet};
use crate::structure::time::Timestamp;
use mio_extras::channel as mio_channel;
use std::fmt;

use std::collections::{HashSet, HashMap};

use std::time::Duration;
use crate::structure::cache_change::CacheChange;

pub struct Reader {
  // Do we need to access information in messageReceiver?? Like reply locators.
  //my_message_receiver: Option<&'mr MessageReceiver>,
  ddsdata_channel: mio_channel::Sender<(DDSData, Timestamp)>,

  history_cache: HistoryCache,
  entity_attributes: EntityAttributes,
  pub enpoint_attributes: EndpointAttributes,

  heartbeat_response_delay: Duration,
  heartbeat_supression_duration: Duration,

  sent_ack_nack_count: i32,
  received_hearbeat_count: i32,

  matched_writers: HashMap<GUID, RtpsWriterProxy>,
} // placeholder

impl Reader {
  pub fn new(guid: GUID, ddsdata_channel: mio_channel::Sender<(DDSData, Timestamp)>) -> Reader {
    Reader {
      ddsdata_channel,
      history_cache: HistoryCache::new(),
      entity_attributes: EntityAttributes { guid },
      enpoint_attributes: EndpointAttributes::default(),

      heartbeat_response_delay: Duration::new(0, 500_000_000), // 0,5sec
      heartbeat_supression_duration: Duration::new(0, 0),
      sent_ack_nack_count: 0,
      received_hearbeat_count: 0,
      matched_writers: HashMap::new(),
    }
  }
  // TODO: check if it's necessary to implement different handlers for discovery
  // and user messages

  // TODO Used for test/debugging purposes
  pub fn get_history_cache_change_data(&self, sequence_number: SequenceNumber) -> Option<DDSData> {
    let cc = self.history_cache.get_change(sequence_number).unwrap();
    println!("history cache !!!! {:?}", cc);
    let pl = cc.data_value.clone();

    match pl {
      Some(xd) => Some(DDSData::from_arc(cc.instance_handle.clone(), xd)),
      None => None,
    }
  }

  // Used for test/debugging purposes
  pub fn get_history_cache_change(&self, sequence_number: SequenceNumber) -> CacheChange {
    println!(
      "history cache !!!! {:?}",
      self.history_cache.get_change(sequence_number).unwrap()
    );
    self
      .history_cache
      .get_change(sequence_number)
      .unwrap()
      .clone()
  }

  // TODO Used for test/debugging purposes
  pub fn get_history_cache_sequence_start_and_end_numbers(&self) -> Vec<SequenceNumber> {
    let start = self.history_cache.get_seq_num_min().unwrap();
    let end = self.history_cache.get_seq_num_max().unwrap();
    return vec![*start, *end];
  }

  fn get_writer_proxy(&mut self, remote_writer_guid: GUID) -> &mut RtpsWriterProxy {
    match self.matched_writers.get_mut(&remote_writer_guid) {
      Some(_) => {}
      None => {
        let new_writer_proxy = RtpsWriterProxy::new(remote_writer_guid);
        self
          .matched_writers
          .insert(remote_writer_guid, new_writer_proxy);
      }
    };
    self.matched_writers.get_mut(&remote_writer_guid).unwrap()
  }

  // handles regular data message and updates history cache
  pub fn handle_data_msg(&mut self, data: Data, timestamp: Timestamp) {
    let user_data = true; // Different action for discovery data?
    if user_data {
      //let writer_proxy = self.get_writer_proxy();
      self.make_cache_change(data);
      self.send_datasample(timestamp);

      self.notify_cache_change();
    } else {
      // is discovery data
      todo!();
    }
  }

  fn send_datasample(&self, timestamp: Timestamp) {
    let cc = self.history_cache.get_latest().unwrap().clone();
    let mut ddsdata = DDSData::from_arc(cc.instance_handle, cc.data_value.unwrap());
    ddsdata.set_reader_id(self.get_guid().entityId.clone());
    ddsdata.set_writer_id(cc.writer_guid.entityId.clone());

    self
      .ddsdata_channel
      .send((ddsdata, timestamp))
      .expect("Unable to send DataSample from Reader");
  }

  // send ack_nack response if necessary. spec page 104
  // Steteless readers shouldn't react to hearbeat_messages
  pub fn handle_heartbeat_msg(
    &mut self,
    heartbeat: Heartbeat,
    final_flag_set: bool,
  ) -> Option<AckNack> {
    // This is done just for stetefull reader. The hearbeat count is maintained
    // separately for all mathched writers
    if heartbeat.count <= self.received_hearbeat_count {
      // Already received newer or same
      return None;
    }
    self.received_hearbeat_count = heartbeat.count;

    // remove fragmented changes untill first_sn ???
    // self.history_cache.remove_changes_up_to(heartbeat.first_sn);
    // self.notify_cache_change();

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
      let mut reader_sn_state = SequenceNumberSet::new(last_seq_num);
      for seq_num in i64::from(last_seq_num)..i64::from(heartbeat.last_sn) {
        reader_sn_state.insert(SequenceNumber::from(seq_num));
      }

      let response_ack_nack = AckNack {
        reader_id: self.get_entity_id(),
        writer_id: heartbeat.writer_id,
        reader_sn_state,
        count: self.sent_ack_nack_count,
      };
      self.sent_ack_nack_count += 1;
      // The messageReceiver has the information on where to send this returned acknack.
      return Some(response_ack_nack);
    }
    None
  }

  pub fn handle_gap_msg(&mut self, gap: Gap) {
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
    let instance_handle = self.history_cache.generate_free_instance_handle();
    let mut ddsdata = DDSData::new(instance_handle, data.serialized_payload);
    let writer_guid =
      GUID::new_with_prefix_and_id(GuidPrefix::GUIDPREFIX_UNKNOWN, data.writer_id.clone());

    ddsdata.set_reader_id(data.reader_id);
    ddsdata.set_writer_id(data.writer_id.clone());

    let change = CacheChange::new(writer_guid, data.writer_sn, Some(ddsdata));
    self.history_cache.add_change(change);
  }

  // notifies DataReaders (or any listeners that history cache has changed for this reader)
  // likely use of mio channel
  fn notify_cache_change(&self) {
    println!("No notification atm...")
    // Do we need some other channel to notify about changes in history_cache?
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
  // Ignores registration and history cache?
  fn eq(&self, other: &Self) -> bool {
    //self.history_cache == other.history_cache &&
    self.entity_attributes == other.entity_attributes
      && self.enpoint_attributes == other.enpoint_attributes
      && self.heartbeat_response_delay == other.heartbeat_response_delay
      && self.heartbeat_supression_duration == other.heartbeat_supression_duration
      && self.sent_ack_nack_count == other.sent_ack_nack_count
      && self.received_hearbeat_count == other.received_hearbeat_count
  }
}

impl fmt::Debug for Reader {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Reader")
      .field("datasample_channel", &"can't print".to_string())
      .field("history_cache", &self.history_cache)
      .field("entity_attributes", &self.entity_attributes)
      .field("enpoint_attributes", &self.enpoint_attributes)
      .field("heartbeat_response_delay", &self.heartbeat_response_delay)
      .field("sent_ack_nack_count", &self.sent_ack_nack_count)
      .field("received_hearbeat_count", &self.received_hearbeat_count)
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::structure::guid::{GUID, EntityId};
  use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;

  #[test]
  fn rtpsreader_send_ddsdata() {
    let guid = GUID::new();

    let (send, rec) = mio_channel::channel::<(DDSData, Timestamp)>();
    let mut reader = Reader::new(guid, send);

    let mut data = Data::default();
    data.reader_id = reader.get_guid().entityId;
    data.writer_id = EntityId::createCustomEntityID([4, 5, 6], 222);

    reader.handle_data_msg(data.clone(), Timestamp::from(time::get_time()));

    let (datasample, _time) = rec.try_recv().unwrap();
    assert_eq!(datasample.data(), data.serialized_payload.value);

    print!("{:?}", datasample.data());
    assert_eq!(*datasample.reader_id(), data.reader_id);
    assert_eq!(*datasample.writer_id(), data.writer_id);
  }

  #[test]
  fn rtpsreader_handle_data() {
    let new_guid = GUID::new();

    let (send, rec) = mio_channel::channel::<(DDSData, Timestamp)>();
    let mut new_reader = Reader::new(new_guid, send);

    let d = Data::new();
    let d_seqnum = SequenceNumber::from(1);
    new_reader.handle_data_msg(d.clone(), Timestamp::TIME_INVALID);

    let (rec_data, _time) = rec.try_recv().unwrap();
    let change = CacheChange::new(
      GUID::new_with_prefix_and_id(GuidPrefix::GUIDPREFIX_UNKNOWN, d.writer_id),
      d_seqnum,
      Some(rec_data),
    );

    assert_eq!(
      new_reader.history_cache.get_change(d_seqnum).unwrap(),
      &change
    );
  }

  #[test]
  fn rtpsreader_handle_heartbeat() {
    let new_guid = GUID::new();

    let (send, _rec) = mio_channel::channel::<(DDSData, Timestamp)>();
    let mut new_reader = Reader::new(new_guid, send);

    let writer_id = EntityId::default();
    let instance_handle = new_reader.history_cache.generate_free_instance_handle();

    let d = DDSData::new(instance_handle, SerializedPayload::new());
    let mut changes = Vec::new();

    let hb_new = Heartbeat {
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // First hearbeat from a new writer
      last_sn: SequenceNumber::from(0),
      count: 1,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_new, true).is_none()); // should be false, no ack

    let hb_one = Heartbeat {
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // Only one in writers cache
      last_sn: SequenceNumber::from(1),
      count: 2,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_one, false).is_some()); // Should send an ack_nack

    // After ack_nack, will receive the following change
    let change = CacheChange::new(
      new_reader.get_guid(),
      SequenceNumber::from(1),
      Some(d.clone()),
    );
    new_reader.history_cache.add_change(change.clone());
    changes.push(change);

    // Duplicate
    let hb_one2 = Heartbeat {
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // Only one in writers cache
      last_sn: SequenceNumber::from(1),
      count: 2,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_one2, false).is_none()); // No acknack

    let hb_3_1 = Heartbeat {
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // writer has last 2 in cache
      last_sn: SequenceNumber::from(3),  // writer has written 3 samples
      count: 3,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_3_1, false).is_some()); // Should send an ack_nack

    // After ack_nack, will receive the following changes
    let change = CacheChange::new(
      new_reader.get_guid(),
      SequenceNumber::from(2),
      Some(d.clone()),
    );
    new_reader.history_cache.add_change(change.clone());
    changes.push(change);
    let change = CacheChange::new(new_reader.get_guid(), SequenceNumber::from(3), Some(d));
    new_reader.history_cache.add_change(change.clone());
    changes.push(change);

    let hb_none = Heartbeat {
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(4), // writer has no samples available
      last_sn: SequenceNumber::from(3),  // writer has written 3 samples
      count: 4,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_none, false).is_some()); // Should sen acknack

    assert_eq!(new_reader.sent_ack_nack_count, 3);
  }

  #[test]
  fn rtpsreader_handle_gap() {
    let new_guid = GUID::new();
    let (send, _rec) = mio_channel::channel::<(DDSData, Timestamp)>();
    let mut reader = Reader::new(new_guid, send);

    let n: i64 = 10;
    let instance_handle = reader.history_cache.generate_free_instance_handle();
    let d = DDSData::new(instance_handle, SerializedPayload::new());
    let mut changes = Vec::new();

    let mut dat = d.clone();
    for i in 0..n {
      let change = CacheChange::new(
        reader.get_guid(),
        SequenceNumber::from(i),
        Some(dat.clone()),
      );
      reader.history_cache.add_change(change.clone());
      dat.instance_key = reader.history_cache.generate_free_instance_handle();
      changes.push(change);
    }

    let writer_id = EntityId::default();

    // make sequence numbers 1-3 and 5 7 irrelevant
    let mut gap_list = SequenceNumberSet::new(SequenceNumber::from(4));
    gap_list.insert(SequenceNumber::from(5 + 4)); // TODO! Why do you need to add base!?
    gap_list.insert(SequenceNumber::from(7 + 4));

    let gap = Gap {
      reader_id: reader.get_entity_id(),
      writer_id,
      gap_start: SequenceNumber::from(1),
      gap_list,
    };

    reader.handle_gap_msg(gap);

    assert_eq!(
      reader.history_cache.get_change(SequenceNumber::from(0)),
      Some(&changes[0])
    );
    assert_eq!(
      reader.history_cache.get_change(SequenceNumber::from(1)),
      None
    );
    assert_eq!(
      reader.history_cache.get_change(SequenceNumber::from(2)),
      None
    );
    assert_eq!(
      reader.history_cache.get_change(SequenceNumber::from(3)),
      None
    );
    assert_eq!(
      reader.history_cache.get_change(SequenceNumber::from(4)),
      Some(&changes[4])
    );
    assert_eq!(
      reader.history_cache.get_change(SequenceNumber::from(5)),
      None
    );
    assert_eq!(
      reader.history_cache.get_change(SequenceNumber::from(6)),
      Some(&changes[6])
    );
    assert_eq!(
      reader.history_cache.get_change(SequenceNumber::from(7)),
      None
    );
    assert_eq!(
      reader.history_cache.get_change(SequenceNumber::from(8)),
      Some(&changes[8])
    );
    assert_eq!(
      reader.history_cache.get_change(SequenceNumber::from(9)),
      Some(&changes[9])
    );
  }
}
