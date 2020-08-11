use crate::structure::entity::Entity;
use crate::structure::endpoint::{Endpoint, EndpointAttributes};
use crate::structure::history_cache::HistoryCache;
use crate::structure::instance_handle::InstanceHandle;
use crate::messages::submessages::data::Data;
use crate::messages::submessages::data_frag::DataFrag;

use crate::dds::ddsdata::DDSData;
use crate::dds::rtps_writer_proxy::RtpsWriterProxy;
use crate::messages::submessages::ack_nack::AckNack;
use crate::messages::submessages::heartbeat::Heartbeat;
use crate::messages::submessages::gap::Gap;
use crate::structure::entity::EntityAttributes;
use crate::structure::guid::GUID;
use crate::structure::sequence_number::{SequenceNumber, SequenceNumberSet};
use crate::structure::time::Timestamp;

use std::sync::{Arc, RwLock};
use crate::structure::dds_cache::{DDSHistoryCache};
use std::time::Instant;

use mio_extras::channel as mio_channel;
use std::fmt;

use std::collections::{HashSet, HashMap};

use std::time::Duration;
use crate::structure::cache_change::CacheChange;
use crate::dds::message_receiver::MessageReceiverState;

pub struct Reader {
  ddsdata_channel: mio_channel::SyncSender<(DDSData, Timestamp)>,

  history_cache: Arc<RwLock<DDSHistoryCache>>,
  seqnum_instant_map: HashMap<SequenceNumber, Instant>,

  history_cache: Arc<RwLock<DDSHistoryCache>>,
  seqnum_instant_map: HashMap<SequenceNumber, Instant>,

  entity_attributes: EntityAttributes,
  pub enpoint_attributes: EndpointAttributes,

  heartbeat_response_delay: Duration,
  heartbeat_supression_duration: Duration,

  sent_ack_nack_count: i32,
  received_hearbeat_count: i32,

  matched_writers: HashMap<GUID, RtpsWriterProxy>,
} // placeholder

impl Reader {
  pub fn new(
    guid: GUID,
    ddsdata_channel: mio_channel::Sender<(DDSData, Timestamp)>,
    history_cache: Arc<RwLock<DDSHistoryCache>>,
  ) -> Reader {
    Reader {
      ddsdata_channel,
      history_cache,
      seqnum_instant_map: HashMap::new(),
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
    let history_cache = self.history_cache.read().unwrap();
    let cc = history_cache.get_change(self.seqnum_instant_map.get(&sequence_number).unwrap());

    println!("history cache !!!! {:?}", cc);

    match cc {
      Some(cc) => Some(DDSData::from_arc(
        cc.instance_handle.clone(),
        cc.data_value.as_ref().unwrap().clone(),
      )),
      None => None,
    }
  }

  // Used for test/debugging purposes
  pub fn get_history_cache_change(&self, sequence_number: SequenceNumber) -> Option<CacheChange> {
    let history_cache = self.history_cache.read().unwrap();
    let cc = history_cache.get_change(self.seqnum_instant_map.get(&sequence_number).unwrap());

    println!("history cache !!!! {:?}", cc);
    match cc {
      Some(cc) => Some(cc.clone()),
      None => None,
    }
  }

  // TODO Used for test/debugging purposes
  pub fn get_history_cache_sequence_start_and_end_numbers(&self) -> Vec<SequenceNumber> {
    let start = self.seqnum_instant_map.iter().min().unwrap().0;
    let end = self.seqnum_instant_map.iter().max().unwrap().0;
    return vec![*start, *end];
  }

  pub fn matched_writer_add(&mut self, remote_writer_guid: GUID, mr_state: MessageReceiverState) {
    match self.matched_writers.get_mut(&remote_writer_guid) {
      Some(_) => {}
      None => {
        let new_writer_proxy = RtpsWriterProxy::new(
          remote_writer_guid,
          mr_state.unicast_reply_locator_list,
          mr_state.multicast_reply_locator_list,
        );
        self
          .matched_writers
          .insert(remote_writer_guid, new_writer_proxy);
      }
    };
  }

  pub fn matched_writer_remove(&mut self, remote_writer_guid: GUID) {
    let writer_proxy = self.matched_writers.remove(&remote_writer_guid).unwrap();
    drop(writer_proxy)
  }

  fn matched_writer_lookup(&mut self, remote_writer_guid: GUID) -> &mut RtpsWriterProxy {
    self.matched_writers.get_mut(&remote_writer_guid).unwrap()
  }

  // handles regular data message and updates history cache
  pub fn handle_data_msg(&mut self, data: Data, mr_state: MessageReceiverState) {
    let writer_guid = GUID::new_with_prefix_and_id(mr_state.source_guid_prefix, data.writer_id);
    let seq_num = data.writer_sn;

    let instant = Instant::now();

    // Really should be checked from qosPolicy?
    // Added in order to test stateless actions. TODO
    let statefull: bool = self.matched_writers.get_mut(&writer_guid).is_some();

    if statefull {
      let writer_proxy = self.matched_writer_lookup(writer_guid);
      if let Some(max_sn) = writer_proxy.available_changes_max() {
        if seq_num <= *max_sn {
          return; // Should be ignored
        }
      }
      // Add the change and get the instant
      writer_proxy.received_changes_add(seq_num, instant);

      // lost changes update? All changes not received with lower seqnum should be marked as lost?
    }

    self.make_cache_change(data, instant, writer_guid);
    // self.send_datasample(mr_state.timestamp);
    self.notify_cache_change();
  }

  fn send_datasample(&self, _timestamp: Timestamp) {
    let cc = self.history_cache.get_latest().unwrap().clone();
    let mut ddsdata = DDSData::from_arc(cc.instance_handle, cc.data_value.unwrap());
    ddsdata.set_reader_id(self.get_guid().entityId.clone());
    ddsdata.set_writer_id(cc.writer_guid.entityId.clone());

    // Sending is accomplished by 1. put datasample in cache 2. send notification to reader
    // 1. Put sample is cache
    // *TODO

    // 2. send notification
    match self.ddsdata_channel.try_send((ddsdata, _timestamp)) {
      Ok(()) => (),                                  // expected result
      Err(mio_channel::TrySendError::Full(_)) => (), // This is harmless. There is a notification in already.
      Err(mio_channel::TrySendError::Disconnected(_)) => {
        // If we get here, our DataReader has died. The Reader should now dispose itself.
        // TODO: Implement Reader disposal.
      }
      Err(mio_channel::TrySendError::Io(_)) => {
        // TODO: What does this mean? Can we ever get here?
      }
    }
  }

  pub fn handle_heartbeat_msg(
    &mut self,
    heartbeat: Heartbeat,
    final_flag_set: bool,
    mr_state: MessageReceiverState,
  ) -> bool {
    let writer_guid =
      GUID::new_with_prefix_and_id(mr_state.source_guid_prefix, heartbeat.writer_id);

    // Added in order to test stateless actions. TODO
    if self.matched_writers.get_mut(&writer_guid).is_none() {
      return false;
    }

    let writer_proxy = self.matched_writer_lookup(writer_guid);

    if heartbeat.count <= writer_proxy.received_heartbeat_count {
      return false;
    }
    writer_proxy.received_heartbeat_count = heartbeat.count;

    // remove fragmented changes until first_sn. Removed a bit later due to self
    // beign borrowd mutably already.
    let removed_instances = writer_proxy.irrelevant_changes_up_to(heartbeat.first_sn);

    // self.notify_cache_change();

    let last_seq_num: SequenceNumber;
    if let Some(num) = writer_proxy.available_changes_max() {
      last_seq_num = *num;
    } else {
      last_seq_num = SequenceNumber::from(0);
    }
    // Remove instances from DDSHistoryCache
    for instant in removed_instances.iter() {
      self
        .history_cache
        .write()
        .unwrap()
        .remove_change(instant)
        .expect("WriterProxy told to remove an instant which was not present");
    }

    // See if ack_nack is needed.
    let changes_missing = last_seq_num < heartbeat.last_sn;
    if changes_missing || !final_flag_set {
      let mut reader_sn_state = SequenceNumberSet::new(last_seq_num);
      for seq_num in i64::from(last_seq_num)..i64::from(heartbeat.last_sn) {
        reader_sn_state.insert(SequenceNumber::from(seq_num));
      }

      let response_ack_nack = AckNack {
        reader_id: self.get_entity_id().clone(),
        writer_id: heartbeat.writer_id,
        reader_sn_state,
        count: self.sent_ack_nack_count,
      };

      self.sent_ack_nack_count += 1;
      // The acknack can be sent now or later. The rest of the RTPS message
      // needs to be constructed. p. 48
      self.send_acknack(response_ack_nack);
      return true;
    }
    false
  }

  pub fn handle_gap_msg(&mut self, gap: Gap, mr_state: MessageReceiverState) {
    // ATM all things related to groups is ignored. TODO?

    let writer_guid = GUID::new_with_prefix_and_id(mr_state.source_guid_prefix, gap.writer_id);
    // Added in order to test stateless actions. TODO
    if self.matched_writers.get_mut(&writer_guid).is_none() {
      return;
    }
    let writer_proxy = self.matched_writer_lookup(writer_guid);

    // Sequencenumber set in the gap is invalid: (section 8.3.5.5)
    if i64::from(gap.gap_start) < 1i64
      || (gap.gap_list.set.iter().max().unwrap() - gap.gap_list.set.iter().min().unwrap()) >= 256
    {
      return;
    }
    // Irrelevant sequence numbers communicated in the Gap message are
    // composed of two groups
    let mut irrelevant_changes_set = HashSet::new();

    // 1. All sequence numbers in the range gapStart <= sequence_number < gapList.base
    for seq_num_i64 in i64::from(gap.gap_start)..i64::from(gap.gap_list.base) {
      irrelevant_changes_set.insert(SequenceNumber::from(seq_num_i64));
    }
    // 2. All the sequence numbers that appear explicitly listed in the gapList.
    for seq_num in &mut gap.gap_list.set.into_iter() {
      irrelevant_changes_set.insert(SequenceNumber::from(seq_num as i64));
    }

    // Remove from writerProxy and DDSHistoryCache
    let mut removed_instances = Vec::new();
    for seq_num in &irrelevant_changes_set {
      removed_instances.push(writer_proxy.irrelevant_changes_set(*seq_num));
      //self.history_cache.remove_change(ih);
    }
    for instant in &removed_instances {
      self.history_cache.write().unwrap().remove_change(instant);
    }

    // Is this needed?
    self.notify_cache_change();
  }

  pub fn handle_datafrag_msg(&mut self, _datafrag: DataFrag, _mr_State: MessageReceiverState) {
    todo!() // comines frags to data which is handled normally. page 51-53
            // let data: Data = something..?
            // self.handle_data_msg(data, mr_state);
  }

  // update history cache
  fn make_cache_change(&mut self, data: Data, instant: Instant, writer_guid: GUID) {
    // Do we need instanceHandles in reader side?
    // How do we determine the instanceHandle? TODO
    let instance_handle = InstanceHandle::default();

    let mut ddsdata = DDSData::new(instance_handle, data.serialized_payload);

    ddsdata.set_reader_id(data.reader_id);
    ddsdata.set_writer_id(data.writer_id);
    let cache_change = CacheChange::new(writer_guid, data.writer_sn, Some(ddsdata));
    self
      .history_cache
      .write()
      .unwrap()
      .add_change(&instant, cache_change);
  }

  // notifies DataReaders (or any listeners that history cache has changed for this reader)
  // likely use of mio channel
  fn notify_cache_change(&self) {
    // TODO!
    println!("No notification atm...")
    // Do we need some other channel to notify about changes in history_cache?
  }

  fn send_acknack(&self, _acknack: AckNack) {
    //todo!()
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
  use crate::structure::guid::GuidPrefix;

  #[test]
  fn rtpsreader_send_ddsdata() {
    let mut guid = GUID::new();
    guid.entityId = EntityId::createCustomEntityID([1, 2, 3], 111);

    let (send, rec) = mio_channel::sync_channel::<(DDSData, Timestamp)>(100);
    let mut reader = Reader::new(guid, send, Arc::new(RwLock::new(DDSHistoryCache::new())));

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };

    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;

    reader.matched_writer_add(writer_guid.clone(), mr_state.clone());

    let mut data = Data::default();
    data.reader_id = EntityId::createCustomEntityID([1, 2, 3], 111);
    data.writer_id = writer_guid.entityId;

    reader.handle_data_msg(data.clone(), mr_state);

    let (datasample, _time) = rec.try_recv().unwrap();
    assert_eq!(datasample.data(), data.serialized_payload.value);

    print!("Received: {:?}", datasample.data());
    print!("Original: {:?}", data.serialized_payload);
    assert_eq!(*datasample.reader_id(), data.reader_id);
    assert_eq!(*datasample.writer_id(), data.writer_id);
  }

  #[test]
  fn rtpsreader_handle_data() {
    let new_guid = GUID::new();

    let (send, rec) = mio_channel::sync_channel::<(DDSData, Timestamp)>(100);
    let mut new_reader = Reader::new(
      new_guid,
      send,
      Arc::new(RwLock::new(DDSHistoryCache::new())),
    );

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };

    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;

    new_reader.matched_writer_add(writer_guid.clone(), mr_state.clone());

    let mut d = Data::new();
    d.writer_id = writer_guid.entityId;
    let d_seqnum = d.writer_sn;
    new_reader.handle_data_msg(d, mr_state);

    let (rec_data, _) = rec.try_recv().unwrap();
    let change = CacheChange::new(writer_guid, d_seqnum, Some(rec_data));

    assert_eq!(
      new_reader.get_history_cache_change(d_seqnum).unwrap(),
      change
    );
  }

  #[test]
  fn rtpsreader_handle_heartbeat() {
    let new_guid = GUID::new();

    let (send, _rec) = mio_channel::sync_channel::<(DDSData, Timestamp)>(100);
    let mut new_reader = Reader::new(
      new_guid,
      send,
      Arc::new(RwLock::new(DDSHistoryCache::new())),
    );

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };

    let writer_id = writer_guid.entityId;

    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;

    new_reader.matched_writer_add(writer_guid.clone(), mr_state.clone());

    let d = DDSData::new(InstanceHandle::default(), SerializedPayload::new());
    let mut changes = Vec::new();

    let hb_new = Heartbeat {
      reader_id: new_reader.get_entity_id().clone(),
      writer_id,
      first_sn: SequenceNumber::from(1), // First hearbeat from a new writer
      last_sn: SequenceNumber::from(0),
      count: 1,
    };
    assert!(!new_reader.handle_heartbeat_msg(hb_new, true, mr_state.clone())); // should be false, no ack

    let hb_one = Heartbeat {
      reader_id: new_reader.get_entity_id().clone(),
      writer_id,
      first_sn: SequenceNumber::from(1), // Only one in writers cache
      last_sn: SequenceNumber::from(1),
      count: 2,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_one, false, mr_state.clone())); // Should send an ack_nack

    // After ack_nack, will receive the following change
    let change = CacheChange::new(
      new_reader.get_guid().clone(),
      SequenceNumber::from(1),
      Some(d.clone()),
    );
    new_reader.history_cache
    .write()
    .unwrap()
    .add_change(&Instant::now(), change);
    changes.push(change);

    // Duplicate
    let hb_one2 = Heartbeat {
      reader_id: new_reader.get_entity_id().clone(),
      writer_id,
      first_sn: SequenceNumber::from(1), // Only one in writers cache
      last_sn: SequenceNumber::from(1),
      count: 2,
    };
    assert!(!new_reader.handle_heartbeat_msg(hb_one2, false, mr_state.clone())); // No acknack

    let hb_3_1 = Heartbeat {
      reader_id: new_reader.get_entity_id().clone(),
      writer_id,
      first_sn: SequenceNumber::from(1), // writer has last 2 in cache
      last_sn: SequenceNumber::from(3),  // writer has written 3 samples
      count: 3,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_3_1, false, mr_state.clone())); // Should send an ack_nack

    // After ack_nack, will receive the following changes
    let change = CacheChange::new(
      new_reader.get_guid().clone(),
      SequenceNumber::from(2),
      Some(d.clone()),
    );
    new_reader.history_cache
    .write()
    .unwrap().add_change(&Instant::now(), change.clone());
    changes.push(change);

    let change = CacheChange::new(new_reader.get_guid(), SequenceNumber::from(3), Some(d));
    new_reader.history_cache
    .write()
    .unwrap().add_change(&Instant::now(), change.clone());
    changes.push(change);

    let hb_none = Heartbeat {
      reader_id: new_reader.get_entity_id().clone(),
      writer_id,
      first_sn: SequenceNumber::from(4), // writer has no samples available
      last_sn: SequenceNumber::from(3),  // writer has written 3 samples
      count: 4,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_none, false, mr_state)); // Should sen acknack

    assert_eq!(new_reader.sent_ack_nack_count, 3);
  }

  #[test]
  fn rtpsreader_handle_gap() {
    let new_guid = GUID::new();
    let (send, _rec) = mio_channel::sync_channel::<(DDSData, Timestamp)>(100);
    let mut reader = Reader::new(
      new_guid,
      send,
      Arc::new(RwLock::new(DDSHistoryCache::new())),
    );

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };
    let writer_id = writer_guid.entityId;

    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;

    reader.matched_writer_add(writer_guid.clone(), mr_state.clone());

    let n: i64 = 10;
    let mut d = Data::new();
    d.writer_id = writer_id;
    let mut changes = Vec::new();

    for i in 0..n {
      d.writer_sn = SequenceNumber::from(i);
      reader.handle_data_msg(d.clone(), mr_state.clone());
      changes.push(reader.get_history_cache_change(d.writer_sn).unwrap().clone());
    }

    // make sequence numbers 1-3 and 5 7 irrelevant
    let mut gap_list = SequenceNumberSet::new(SequenceNumber::from(4));
    gap_list.insert(SequenceNumber::from(5 + 4)); // TODO! Why do you need to add base!?
    gap_list.insert(SequenceNumber::from(7 + 4));

    let gap = Gap {
      reader_id: reader.get_entity_id().clone(),
      writer_id,
      gap_start: SequenceNumber::from(1),
      gap_list,
    };

    // Cache changee muutetaan tutkiin datan kirjoittajaa.
    reader.handle_gap_msg(gap, mr_state);

    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(0)),
      Some(changes[0])
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(1)),
      None
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(2)),
      None
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(3)),
      None
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(4)),
      Some(changes[4])
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(5)),
      None
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(6)),
      Some(changes[6])
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(7)),
      None
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(8)),
      Some(changes[8])
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(9)),
      Some(changes[9])
    );
  }
}
