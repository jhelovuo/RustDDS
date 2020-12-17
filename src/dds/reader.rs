use crate::{
  common::timed_event_handler::TimedEventHandler,
  network::constant::TimerMessageType,
  structure::{cache_change::ChangeKind, entity::Entity},
};
use crate::structure::endpoint::{Endpoint, EndpointAttributes};
use crate::messages::submessages::submessages::*;

use crate::dds::ddsdata::DDSData;
use crate::dds::rtps_writer_proxy::RtpsWriterProxy;
use crate::structure::guid::{GUID, EntityId};
use crate::structure::sequence_number::{SequenceNumber, SequenceNumberSet};
use crate::structure::locator::LocatorList;
use crate::structure::{duration::Duration, time::Timestamp};

use std::{
  collections::hash_map::DefaultHasher,
  hash::Hasher,
  slice::Iter,
  sync::{Arc, RwLock},
};
use crate::structure::dds_cache::{DDSCache};
//use std::time::Instant;

use mio::Token;
use mio_extras::channel as mio_channel;
use log::{debug, info, warn};
use std::fmt;

use std::collections::{HashSet, HashMap};
use std::time::Duration as StdDuration;
use enumflags2::BitFlags;

use crate::structure::cache_change::CacheChange;
use crate::dds::message_receiver::MessageReceiverState;
use crate::dds::qos::{QosPolicies, HasQoSPolicy};
use crate::network::udp_sender::UDPSender;

use crate::serialization::message::Message;
use crate::messages::header::Header;
use crate::messages::protocol_id::ProtocolId;
use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::vendor_id::VendorId;
use speedy::{Writable, Endianness};
use chrono::Duration as chronoDuration;

use super::{
  qos::{QosPolicyBuilder},
  values::result::{RequestedDeadlineMissedStatus, StatusChange},
  with_key::datareader::ReaderCommand,
};

use super::qos::InlineQos;


pub(crate) struct Reader {
  // Should the instant be sent?
  notification_sender: mio_channel::SyncSender<()>,
  status_sender: mio_channel::SyncSender<StatusChange>,

  dds_cache: Arc<RwLock<DDSCache>>,
  seqnum_instant_map: HashMap<SequenceNumber, Timestamp>,
  topic_name: String,
  qos_policy: QosPolicies,

  my_guid: GUID,
  pub enpoint_attributes: EndpointAttributes,

  heartbeat_response_delay: StdDuration,
  heartbeat_supression_duration: StdDuration,

  sent_ack_nack_count: i32,
  received_hearbeat_count: i32,

  matched_writers: HashMap<GUID, RtpsWriterProxy>,

  requested_deadline_missed_status: RequestedDeadlineMissedStatus,

  timed_event_handler: Option<TimedEventHandler>,
  pub(crate) data_reader_command_receiver: mio_channel::Receiver<ReaderCommand>,
} // placeholder

impl Reader {
  pub fn new(
    guid: GUID,
    notification_sender: mio_channel::SyncSender<()>,
    status_sender: mio_channel::SyncSender<StatusChange>,
    dds_cache: Arc<RwLock<DDSCache>>,
    topic_name: String,
    data_reader_command_receiver: mio_channel::Receiver<ReaderCommand>, //qos_policy: QosPolicies, add later to constructor
  ) -> Reader {
    Reader {
      notification_sender,
      status_sender,
      dds_cache,
      topic_name,
      qos_policy: QosPolicyBuilder::new().build(),

      seqnum_instant_map: HashMap::new(),
      my_guid: guid ,
      enpoint_attributes: EndpointAttributes::default(),

      heartbeat_response_delay: StdDuration::new(0, 500_000_000), // 0,5sec
      heartbeat_supression_duration: StdDuration::new(0, 0),
      sent_ack_nack_count: 0,
      received_hearbeat_count: 0,
      matched_writers: HashMap::new(),
      requested_deadline_missed_status: RequestedDeadlineMissedStatus::new(),
      timed_event_handler: None,
      data_reader_command_receiver,
    }
  }
  // TODO: check if it's necessary to implement different handlers for discovery
  // and user messages

  // TODO Used for test/debugging purposes
  pub fn get_history_cache_change_data(&self, sequence_number: SequenceNumber) -> Option<DDSData> {
    let dds_cache = self.dds_cache.read().unwrap();
    let cc = dds_cache.from_topic_get_change(
      &self.topic_name,
      &self.seqnum_instant_map.get(&sequence_number).unwrap(),
    );

    debug!("history cache !!!! {:?}", cc);

    match cc {
      Some(cc) => Some(DDSData::new(cc.data_value.as_ref().unwrap().clone())),
      None => None,
    }
  }

  /// To know when token represents a reader we should look entity attribute kind
  pub fn get_entity_token(&self) -> Token {
    let id = self.get_guid().as_usize();
    Token(id)
  }

  pub fn get_reader_command_entity_token(&self) -> Token {
    let mut hasher = DefaultHasher::new();
    let id = self.get_guid().as_usize() as u64;
    hasher.write(&id.to_le_bytes());
    let hashedID: u64 = hasher.finish();
    Token(hashedID as usize)
  }

  pub fn add_timed_event_handler(&mut self, time_handler: TimedEventHandler) {
    self.timed_event_handler = Some(time_handler);
  }

  pub fn set_requested_deadline_check_timer(&mut self) {
    if let Some(deadline) = self.qos_policy.deadline {
      debug!(
        "set_requested_deadline_check_timer: {:?}",
        deadline.0.to_std()
      );
      match chronoDuration::from_std(deadline.0.to_std()) {
        Ok(cdur) => match self.timed_event_handler.as_mut() {
          Some(teh) => teh.set_timeout(&cdur, TimerMessageType::reader_deadline_missed_check),
          None => warn!("Unable to get timed_event_handler."),
        },
        Err(_) => {
          warn!("Failed to get chrono duration from deadline {:?}", deadline);
        }
      }
    } else {
      debug!("do not set set_requested_deadline_check_timer")
    }
  }

  pub fn reset_requested_deadline_missed_status(&mut self) {
    info!(
      "reset_requested_deadline_missed_status on reader {:?}",
      self.get_entity_id()
    );
    self.requested_deadline_missed_status.reset_change();
  }

  pub fn send_status_change(&self, change: StatusChange) {
    match self.status_sender.try_send(change.clone()) {
      Ok(()) => info!(
        "Reader {:?} send status change: {:?}",
        self.get_entity_id(),
        change.clone()
      ),
      Err(mio_channel::TrySendError::Full(_)) => {
        warn!("Reader cannot send new status changes, datareader is full.")
      }
      Err(mio_channel::TrySendError::Disconnected(_)) => {
        // If we get here, our DataReader has died. The Reader should now dispose itself.
        // TODO: Implement Reader disposal.
        panic!()
      }
      Err(mio_channel::TrySendError::Io(_)) => {
        // TODO: What does this mean? Can we ever get here?
        panic!()
      }
    }
  }

  // The deadline that the DataReader was expecting through its QosPolicy
  // DEADLINE was not respected for a specific instance
  // if statusChange is returned it should be send to DataReader
  // this calculation should be repeated every self.qos_policy.deadline
  fn calculate_if_requested_deadline_is_missed(&mut self) -> Vec<StatusChange> {
    debug!("calculate_if_requested_deadline_is_missed");
    let mut changes: Vec<StatusChange> = vec![];
    match self.qos_policy.deadline {
      None => {
        return changes;
      }
      Some(deadline) => {
        for (_g, writer_proxy) in self.matched_writers.iter_mut() {
          //let last_instant = wP.changes.values().max_by(|x,y|x.cmp(y));
          let last_instant = writer_proxy.changes.values().max_by(|x, y| x.cmp(y));
          match last_instant {
            Some(instant) => {
              let insta_now = Timestamp::now();
              let perioid = insta_now.duration_since(*instant);
              // if time singe last received message is greater than deadline increase status and return notification.
              debug!("Comparing deadlines: {:?} - {:?}", perioid, deadline);
              if perioid > deadline.0 {
                debug!("Deadline missed: {:?} - {:?}", perioid, deadline);
                self.requested_deadline_missed_status.increase();
                changes.push(StatusChange::RequestedDeadlineMissedStatus(
                  self.requested_deadline_missed_status,
                ));
              } else {
                continue;
              }
              // no messages recieved ever so deadline must be missed.
            }
            None => {
              self.requested_deadline_missed_status.increase();
              changes.push(StatusChange::RequestedDeadlineMissedStatus(
                self.requested_deadline_missed_status,
              ));
            }
          }
        }
      }
    }
    changes
  }

  /*
  fn calculate_if_requested_deadline_is_missed_for_specific_writer(&mut self, writer_proxy: &RtpsWriterProxy) -> Option<StatusChange>{
    let last_instant = writer_proxy.changes.values().max_by(|x,y|x.cmp(y));
    match last_instant{
      Some(instant) => {
        let insta_now = Instant::now();
        let perioid = insta_now.duration_since(*instant);
        // if time singe last received message is greater than deadline increase status and return notification.
        if perioid > self.qos_policy.deadline.unwrap().period.to_std(){
          self.requested_deadline_missed_status.increase();
          return Some( StatusChange::RequestedDeadlineMissedStatus{ status : self.requested_deadline_missed_status} );
        }else{
          return None;
        }
      // no messages recieved ever so deadline must be missed.
      } None => {
        self.requested_deadline_missed_status.increase();
        return Some( StatusChange::RequestedDeadlineMissedStatus{ status : self.requested_deadline_missed_status} );
      }
    }
  }
  */

  pub fn handle_requested_deadline_event(&mut self) {
    debug!("handle_requested_deadline_event");
    let missed_deadlines = self.calculate_if_requested_deadline_is_missed();
    for missed_deadline in missed_deadlines {
      self.send_status_change(missed_deadline);
    }
    self.set_requested_deadline_check_timer();
  }

  // Used for test/debugging purposes
  pub fn get_history_cache_change(&self, sequence_number: SequenceNumber) -> Option<CacheChange> {
    debug!("{:?}", sequence_number);
    let dds_cache = self.dds_cache.read().unwrap();
    let cc = dds_cache.from_topic_get_change(
      &self.topic_name,
      &self.seqnum_instant_map.get(&sequence_number).unwrap(),
    );
    debug!("history cache !!!! {:?}", cc);
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

  // updates or adds a new writer proxy, doesn't touch changes
  pub fn add_writer_proxy(&mut self, proxy: RtpsWriterProxy) {
    let old_proxy = self.matched_writer_lookup(proxy.remote_writer_guid);
    match old_proxy {
      Some(op) => op.update_contents(proxy),
      None => {
        self.matched_writers.insert(proxy.remote_writer_guid, proxy);
      }
    };
  }

  pub fn contains_writer(&self, entity_id: EntityId) -> bool {
    self
      .matched_writers
      .iter()
      .find(|(&g, _)| g.entityId == entity_id)
      .is_some()
  }

  pub fn matched_writer_add(
    &mut self,
    remote_writer_guid: GUID,
    remote_group_entity_id: EntityId,
    unicast_locator_list: LocatorList,
    multicast_locator_list: LocatorList,
  ) {
    let proxy = RtpsWriterProxy::new(
      remote_writer_guid,
      unicast_locator_list,
      multicast_locator_list,
      remote_group_entity_id,
    );
    self.add_writer_proxy(proxy);
  }

  pub fn retain_matched_writers(&mut self, retvals: Iter<RtpsWriterProxy>) {
    let rt: Vec<GUID> = retvals.map(|p| p.remote_writer_guid).collect();
    self.matched_writers.retain(|guid, _| rt.contains(guid));
  }

  pub fn matched_writer_remove(&mut self, remote_writer_guid: GUID) -> Option<RtpsWriterProxy> {
    self.matched_writers.remove(&remote_writer_guid)
  }

  fn matched_writer_lookup(&mut self, remote_writer_guid: GUID) -> Option<&mut RtpsWriterProxy> {
    self.matched_writers.get_mut(&remote_writer_guid)
  }

  // handles regular data message and updates history cache
  pub fn handle_data_msg(&mut self, data: Data, mr_state: MessageReceiverState) {
    let duration = match mr_state.timestamp {
      Some(ts) => Timestamp::now().duration_since(ts),
      None => Duration::DURATION_ZERO,
    };

    // checking lifespan for silent dropping of message
    match self.get_qos().lifespan {
      Some(ls) => {
        if ls.duration < duration {
          return;
        }
      }
      None => (),
    }

    let writer_guid = GUID::new_with_prefix_and_id(mr_state.source_guid_prefix, data.writer_id);
    let seq_num = data.writer_sn;

    let instant = Timestamp::now();

    // Really should be checked from qosPolicy?
    // Added in order to test stateless actions.
    // TODO
    let statefull = self.matched_writers.contains_key(&writer_guid);

    let mut no_writers = false;

    if statefull {
      if let Some(writer_proxy) = self.matched_writer_lookup(writer_guid) {
        if writer_proxy.contains_change(seq_num) {
          // change already present
          return;
        }
        // Add the change and get the instant
        writer_proxy.received_changes_add(seq_num, instant);
      } else {
        no_writers = true;
      }
    }

    self.make_cache_change(data, instant, writer_guid, no_writers);
    // Add to own track-keeping datastructure
    self.seqnum_instant_map.insert(seq_num, instant);

    self.notify_cache_change();
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
    if !self.matched_writers.contains_key(&writer_guid) {
      return false;
    }

    let writer_proxy = match self.matched_writer_lookup(writer_guid) {
      Some(wp) => wp,
      None => return false, // Matching writer not found
    };

    let mut mr_state = mr_state;
    mr_state.unicast_reply_locator_list = writer_proxy.unicast_locator_list.clone();

    if heartbeat.count <= writer_proxy.received_heartbeat_count {
      return false;
    }
    writer_proxy.received_heartbeat_count = heartbeat.count;

    // remove fragmented changes until first_sn.
    let removed_instances = writer_proxy.irrelevant_changes_up_to(heartbeat.first_sn);

    // Remove instances from DDSHistoryCache
    let mut cache = match self.dds_cache.write() {
      Ok(rwlock) => rwlock,
      // TODO: Should we panic here? Are we allowed to continue with poisoned DDSCache?
      Err(e) => panic!("The DDSCache of is poisoned. Error: {}", e),
    };
    for instant in removed_instances.iter() {
      match cache.from_topic_remove_change(&self.topic_name, instant) {
        Some(_) => (),
        None => warn!("WriterProxy told to remove an instant which was not present"),
      }
    }
    drop(cache);

    let writer_proxy = match self.matched_writer_lookup(writer_guid) {
      Some(wp) => wp,
      None => return false, // Matching writer not found
    };
    // See if ack_nack is needed.
    if writer_proxy.changes_are_missing(heartbeat.first_sn, heartbeat.last_sn) || !final_flag_set {
      let missing_seqnums =
        writer_proxy.get_missing_sequence_numbers(heartbeat.first_sn, heartbeat.last_sn);
      let seqnum_base = missing_seqnums.iter().min();
      let mut seqnum_set = match seqnum_base {
        Some(&base) => SequenceNumberSet::new(base),
        None => SequenceNumberSet::new(heartbeat.last_sn),
      };

      for seqnum in missing_seqnums {
        seqnum_set.insert(seqnum);
      }

      let response_ack_nack = AckNack {
        reader_id: self.get_entity_id(),
        writer_id: heartbeat.writer_id,
        reader_sn_state: seqnum_set,
        count: self.sent_ack_nack_count,
      };

      self.sent_ack_nack_count += 1;
      // The acknack can be sent now or later. The rest of the RTPS message
      // needs to be constructed. p. 48

      self.send_acknack(response_ack_nack, mr_state);
      return true;
    }
    false
  }

  pub fn handle_gap_msg(&mut self, gap: Gap, mr_state: MessageReceiverState) {
    // ATM all things related to groups is ignored. TODO?

    let writer_guid = GUID::new_with_prefix_and_id(mr_state.source_guid_prefix, gap.writer_id);
    // Added in order to test stateless actions. TODO

    let writer_proxy = match self.matched_writer_lookup(writer_guid) {
      Some(wp) => wp,
      None => return, // Matching writer not found
    };

    // Sequencenumber set in the gap is invalid: (section 8.3.5.5)
    // Set checked to be not empty. Unwraps won't panic
    if gap.gap_list.set.len() != 0 {
      if i64::from(gap.gap_start) < 1i64
        || (gap.gap_list.set.iter().max().unwrap() - gap.gap_list.set.iter().min().unwrap()) >= 256
      {
        return;
      }
    } else {
      return;
    };
    // Irrelevant sequence numbers communicated in the Gap message are
    // composed of two groups:
    let mut irrelevant_changes_set = HashSet::new();

    //   1. All sequence numbers in the range gapStart <= sequence_number < gapList.base
    for seq_num_i64 in i64::from(gap.gap_start)..i64::from(gap.gap_list.base) {
      irrelevant_changes_set.insert(SequenceNumber::from(seq_num_i64));
    }
    //   2. All the sequence numbers that appear explicitly listed in the gapList.
    for seq_num in &mut gap.gap_list.set.into_iter() {
      irrelevant_changes_set.insert(SequenceNumber::from(seq_num as i64));
    }

    // Remove from writerProxy and DDSHistoryCache
    let mut removed_instances = Vec::new();
    for seq_num in &irrelevant_changes_set {
      match writer_proxy.set_irrelevant_change(*seq_num) {
        Some(i) => removed_instances.push(i),
        None => (),
      };
    }
    let mut cache = match self.dds_cache.write() {
      Ok(rwlock) => rwlock,
      // TODO: Should we panic here? Are we allowed to continue with poisoned DDSCache?
      Err(e) => panic!("The DDSCache of is poisoned. Error: {}", e),
    };
    for instant in &removed_instances {
      cache.from_topic_remove_change(&self.topic_name, instant);
    }

    // Is this needed?
    // self.notify_cache_change();
  }

  pub fn handle_datafrag_msg(&mut self, _datafrag: DataFrag, _mr_State: MessageReceiverState) {
    todo!() // comines frags to data which is handled normally. page 51-53
            // let data: Data = something..?
            // self.handle_data_msg(data, mr_state);
  }

  pub fn handle_heartbeatfrag_msg(
    &mut self,
    _heartbeatfrag: HeartbeatFrag,
    _mr_state: MessageReceiverState,
  ) {
    todo!()
  }

  // update history cache
  fn make_cache_change(
    &mut self,
    data: Data,
    instant: Timestamp,
    writer_guid: GUID,
    no_writers: bool,
  ) {
    let representation_identifier = match &data.serialized_payload {
      Some(sp) => sp.representation_identifier(),
      None => RepresentationIdentifier::CDR_LE,
    };

    let status_info = match &data.inline_qos {
      Some(iqos) => InlineQos::status_info(iqos, representation_identifier).ok(),
      None => None,
    };

    let key_hash = match &data.inline_qos {
      Some(iqos) => InlineQos::key_hash(iqos, representation_identifier).ok(),
      None => None,
    };

    let change_kind = match status_info {
      Some(si) => si.change_kind(),
      None => {
        if !no_writers {
          ChangeKind::ALIVE
        } else {
          ChangeKind::NOT_ALIVE_UNREGISTERED
        }
      }
    };

    if change_kind != ChangeKind::ALIVE {
      debug!(
        "Changed writer {:?} status to {:?}",
        writer_guid, change_kind
      );
    }

    let mut ddsdata = if change_kind != ChangeKind::ALIVE {
      DDSData::new_disposed(status_info, key_hash)
    } else {
      match data.serialized_payload {
        Some(pl) => DDSData::new(pl),
        None => return,
      }
    };

    ddsdata.set_reader_id(data.reader_id);
    ddsdata.set_writer_id(data.writer_id);
    let cache_change = CacheChange::new(change_kind, writer_guid, data.writer_sn, Some(ddsdata));
    let mut cache = match self.dds_cache.write() {
      Ok(rwlock) => rwlock,
      // TODO: Should we panic here? Are we allowed to continue with poisoned DDSCache?
      Err(e) => panic!("The DDSCache of is poisoned. Error: {}", e),
    };
    cache.to_topic_add_change(&self.topic_name, &instant, cache_change);
  }

  // notifies DataReaders (or any listeners that history cache has changed for this reader)
  // likely use of mio channel
  fn notify_cache_change(&self) {
    match self.notification_sender.try_send(()) {
      Ok(()) => (),
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

  fn send_acknack(&self, acknack: AckNack, mr_state: MessageReceiverState) {
    // Should it be saved as an attribute?
    let sender = UDPSender::new_with_random_port();
    // TODO: How to determine which flags should be one? Both on atm
    let flags = BitFlags::<ACKNACK_Flags>::from_flag(ACKNACK_Flags::Endianness)
      | BitFlags::<ACKNACK_Flags>::from_flag(ACKNACK_Flags::Final);

    let infodst_flags =
      BitFlags::<INFODESTINATION_Flags>::from_flag(INFODESTINATION_Flags::Endianness);

    let mut message = Message::new(Header {
      protocol_id: ProtocolId::default(),
      protocol_version: ProtocolVersion::THIS_IMPLEMENTATION,
      vendor_id: VendorId::THIS_IMPLEMENTATION,
      guid_prefix: self.my_guid.guidPrefix,
    });

    let info_dst = InfoDestination {
      guid_prefix: mr_state.source_guid_prefix,
    };

    match info_dst.create_submessage(infodst_flags) {
      Some(m) => message.add_submessage(m),
      None => return,
    };

    match acknack.create_submessage(flags) {
      Some(m) => message.add_submessage(m),
      None => return,
    };

    /*let mut bytes = message.serialize_header();
    bytes.extend_from_slice(&submessage.serialize_msg()); */
    let bytes = message
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .unwrap();
    sender.send_to_locator_list(&bytes, &mr_state.unicast_reply_locator_list);
  }

  pub fn send_preemptive_acknacks(&mut self) {
    let sender = UDPSender::new_with_random_port();

    let flags = BitFlags::<ACKNACK_Flags>::from_flag(ACKNACK_Flags::Endianness)
      | BitFlags::<ACKNACK_Flags>::from_flag(ACKNACK_Flags::Final);

    let infodst_flags =
      BitFlags::<INFODESTINATION_Flags>::from_flag(INFODESTINATION_Flags::Endianness);

    self.sent_ack_nack_count += 1;

    for (_, writer_proxy) in self
      .matched_writers
      .iter()
      .filter(|(_, p)| p.changes.is_empty())
    {
      let mut message = Message::new(Header {
        protocol_id: ProtocolId::default(),
        protocol_version: ProtocolVersion::THIS_IMPLEMENTATION,
        vendor_id: VendorId::THIS_IMPLEMENTATION,
        guid_prefix: self.my_guid.guidPrefix,
      });

      let info_dst = InfoDestination {
        guid_prefix: writer_proxy.remote_writer_guid.guidPrefix,
      };

      let acknack = AckNack {
        reader_id: self.get_entity_id(),
        writer_id: writer_proxy.remote_writer_guid.entityId,
        reader_sn_state: SequenceNumberSet::new(SequenceNumber::from(1)),
        count: self.sent_ack_nack_count,
      };

      match info_dst.create_submessage(infodst_flags) {
        Some(m) => message.add_submessage(m),
        None => continue,
      };

      match acknack.create_submessage(flags) {
        Some(m) => message.add_submessage(m),
        None => continue,
      };

      let bytes = message
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap();
      sender.send_to_locator_list(&bytes, &writer_proxy.unicast_locator_list);
    }
  }

  pub fn topic_name(&self) -> &String {
    &self.topic_name
  }
} // impl

impl HasQoSPolicy for Reader {
  // fn set_qos(&mut self, policy: &QosPolicies) -> DDSResult<()> {
  //   // TODO: check liveliness of qos_policy
  //   self.qos_policy = policy.clone();
  //   Ok(())
  // }

  fn get_qos(&self) -> QosPolicies {
    self.qos_policy.clone()
  }
}

impl Entity for Reader {
  fn get_guid(&self) -> GUID {
    self.my_guid
  }
}

impl Endpoint for Reader {
  fn as_endpoint(&self) -> &crate::structure::endpoint::EndpointAttributes {
    &self.enpoint_attributes
  }
}

// Not needed anymore
/*impl PartialEq for Reader {
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
}*/

impl fmt::Debug for Reader {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Reader")
      .field("notification_sender, dds_cache", &"can't print".to_string())
      .field("topic_name", &self.topic_name)
      .field("my_guid", &self.my_guid)
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
  use crate::{
    dds::values::result::StatusChange,
    structure::guid::{GUID, EntityId},
  };
  use crate::messages::submessages::submessage_elements::serialized_payload::{SerializedPayload};
  use crate::structure::guid::GuidPrefix;
  use crate::structure::topic_kind::TopicKind;
  use crate::dds::typedesc::TypeDesc;

  #[test]
  fn rtpsreader_notification() {
    let mut guid = GUID::new();
    guid.entityId = EntityId::createCustomEntityID([1, 2, 3], 111);

    let (send, rec) = mio_channel::sync_channel::<()>(100);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<StatusChange>(100);
    let (_reader_command_sender, reader_command_receiver) =
      mio_channel::sync_channel::<ReaderCommand>(10);

    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));
    dds_cache.write().unwrap().add_new_topic(
      &"test".to_string(),
      TopicKind::NoKey,
      TypeDesc::new("testi".to_string()),
    );
    let mut reader = Reader::new(
      guid,
      send,
      status_sender,
      dds_cache,
      "test".to_string(),
      reader_command_receiver,
    );

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };

    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;

    reader.matched_writer_add(
      writer_guid.clone(),
      EntityId::ENTITYID_UNKNOWN,
      mr_state.unicast_reply_locator_list.clone(),
      mr_state.multicast_reply_locator_list.clone(),
    );

    let mut data = Data::default();
    data.reader_id = EntityId::createCustomEntityID([1, 2, 3], 111);
    data.writer_id = writer_guid.entityId;

    reader.handle_data_msg(data, mr_state);

    assert!(rec.try_recv().is_ok());
  }

  #[test]
  fn rtpsreader_handle_data() {
    let new_guid = GUID::new();

    let (send, rec) = mio_channel::sync_channel::<()>(100);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<StatusChange>(100);
    let (_reader_command_sender, reader_command_receiver) =
      mio_channel::sync_channel::<ReaderCommand>(10);

    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));
    dds_cache.write().unwrap().add_new_topic(
      &"test".to_string(),
      TopicKind::NoKey,
      TypeDesc::new("testi".to_string()),
    );
    let mut new_reader = Reader::new(
      new_guid,
      send,
      status_sender,
      dds_cache.clone(),
      "test".to_string(),
      reader_command_receiver,
    );

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };

    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;

    new_reader.matched_writer_add(
      writer_guid.clone(),
      EntityId::ENTITYID_UNKNOWN,
      mr_state.unicast_reply_locator_list.clone(),
      mr_state.multicast_reply_locator_list.clone(),
    );

    let mut d = Data::default();
    d.writer_id = writer_guid.entityId;
    let d_seqnum = d.writer_sn;
    new_reader.handle_data_msg(d.clone(), mr_state);

    assert!(rec.try_recv().is_ok());

    let hc_locked = dds_cache.read().unwrap();
    let cc_from_chache = hc_locked.from_topic_get_change(
      &new_reader.topic_name,
      &new_reader.seqnum_instant_map.get(&d_seqnum).unwrap(),
    );

    let ddsdata = DDSData::new(d.serialized_payload.unwrap());
    let cc_built_here = CacheChange::new(ChangeKind::ALIVE, writer_guid, d_seqnum, Some(ddsdata));

    assert_eq!(cc_from_chache.unwrap(), &cc_built_here);
  }

  #[test]
  fn rtpsreader_handle_heartbeat() {
    let new_guid = GUID::new();

    let (send, _rec) = mio_channel::sync_channel::<()>(100);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<StatusChange>(100);
    let (_reader_command_sender, reader_command_receiver) =
      mio_channel::sync_channel::<ReaderCommand>(10);

    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));
    dds_cache.write().unwrap().add_new_topic(
      &"test".to_string(),
      TopicKind::NoKey,
      TypeDesc::new("testi".to_string()),
    );
    let mut new_reader = Reader::new(
      new_guid,
      send,
      status_sender,
      dds_cache,
      "test".to_string(),
      reader_command_receiver,
    );

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };

    let writer_id = writer_guid.entityId;

    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;

    new_reader.matched_writer_add(
      writer_guid.clone(),
      EntityId::ENTITYID_UNKNOWN,
      mr_state.unicast_reply_locator_list.clone(),
      mr_state.multicast_reply_locator_list.clone(),
    );

    let d = DDSData::new(SerializedPayload::default());
    let mut changes = Vec::new();

    let hb_new = Heartbeat {
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // First hearbeat from a new writer
      last_sn: SequenceNumber::from(0),
      count: 1,
    };
    assert!(!new_reader.handle_heartbeat_msg(hb_new, true, mr_state.clone())); // should be false, no ack

    let hb_one = Heartbeat {
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // Only one in writers cache
      last_sn: SequenceNumber::from(1),
      count: 2,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_one, false, mr_state.clone())); // Should send an ack_nack

    // After ack_nack, will receive the following change
    let change = CacheChange::new(
      ChangeKind::ALIVE,
      new_reader.get_guid(),
      SequenceNumber::from(1),
      Some(d.clone()),
    );
    new_reader.dds_cache.write().unwrap().to_topic_add_change(
      &new_reader.topic_name,
      &Timestamp::now(),
      change.clone(),
    );
    changes.push(change);

    // Duplicate
    let hb_one2 = Heartbeat {
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // Only one in writers cache
      last_sn: SequenceNumber::from(1),
      count: 2,
    };
    assert!(!new_reader.handle_heartbeat_msg(hb_one2, false, mr_state.clone())); // No acknack

    let hb_3_1 = Heartbeat {
      reader_id: new_reader.get_entity_id(),
      writer_id,
      first_sn: SequenceNumber::from(1), // writer has last 2 in cache
      last_sn: SequenceNumber::from(3),  // writer has written 3 samples
      count: 3,
    };
    assert!(new_reader.handle_heartbeat_msg(hb_3_1, false, mr_state.clone())); // Should send an ack_nack

    // After ack_nack, will receive the following changes
    let change = CacheChange::new(
      ChangeKind::ALIVE,
      new_reader.get_guid(),
      SequenceNumber::from(2),
      Some(d.clone()),
    );
    new_reader.dds_cache.write().unwrap().to_topic_add_change(
      &new_reader.topic_name,
      &Timestamp::now(),
      change.clone(),
    );
    changes.push(change);

    let change = CacheChange::new(
      ChangeKind::ALIVE,
      new_reader.get_guid(),
      SequenceNumber::from(3),
      Some(d),
    );
    new_reader.dds_cache.write().unwrap().to_topic_add_change(
      &new_reader.topic_name,
      &Timestamp::now(),
      change.clone(),
    );
    changes.push(change);

    let hb_none = Heartbeat {
      reader_id: new_reader.get_entity_id(),
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
    let (send, _rec) = mio_channel::sync_channel::<()>(100);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<StatusChange>(100);
    let (_reader_command_sender, reader_command_receiver) =
      mio_channel::sync_channel::<ReaderCommand>(10);

    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));
    dds_cache.write().unwrap().add_new_topic(
      &"test".to_string(),
      TopicKind::NoKey,
      TypeDesc::new("testi".to_string()),
    );
    let mut reader = Reader::new(
      new_guid,
      send,
      status_sender,
      dds_cache,
      "test".to_string(),
      reader_command_receiver,
    );

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };
    let writer_id = writer_guid.entityId;

    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;

    reader.matched_writer_add(
      writer_guid.clone(),
      EntityId::ENTITYID_UNKNOWN,
      mr_state.unicast_reply_locator_list.clone(),
      mr_state.multicast_reply_locator_list.clone(),
    );

    let n: i64 = 10;
    let mut d = Data::default();
    d.writer_id = writer_id;
    let mut changes = Vec::new();

    for i in 0..n {
      d.writer_sn = SequenceNumber::from(i);
      reader.handle_data_msg(d.clone(), mr_state.clone());
      changes.push(
        reader
          .get_history_cache_change(d.writer_sn)
          .unwrap()
          .clone(),
      );
    }

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

    // Cache changee muutetaan tutkiin datan kirjoittajaa.
    reader.handle_gap_msg(gap, mr_state);

    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(0)),
      Some(changes[0].clone())
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
      Some(changes[4].clone())
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(5)),
      None
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(6)),
      Some(changes[6].clone())
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(7)),
      None
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(8)),
      Some(changes[8].clone())
    );
    assert_eq!(
      reader.get_history_cache_change(SequenceNumber::from(9)),
      Some(changes[9].clone())
    );
  }
}
