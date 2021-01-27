use chrono::Duration as chronoDuration;


#[allow(unused_imports)]
use log::{debug, error, warn, info, trace};

use speedy::{Writable, Endianness};
use mio_extras::channel::{self as mio_channel, SyncSender};
use mio::Token;
use std::{
  sync::{RwLock, Arc},
  collections::{HashSet, HashMap, BTreeMap, BTreeSet, hash_map::DefaultHasher},
  iter::FromIterator,
  cmp::max,
};
use std::hash::Hasher;

use crate::{
  serialization::MessageBuilder,
};

use crate::structure::time::Timestamp;
use crate::structure::duration::Duration;

use crate::structure::guid::{GuidPrefix, EntityId, GUID};
use crate::structure::sequence_number::{SequenceNumber};
use crate::{
  messages::submessages::submessages::{
    AckNack,
  },
  structure::cache_change::{CacheChange},
  serialization::{Message},
  dds::dp_event_loop::NACK_RESPONSE_DELAY,
};

use crate::dds::{ddsdata::DDSData, qos::HasQoSPolicy};
use crate::{
  network::{constant::TimerMessageType, udp_sender::UDPSender},
  structure::{
    entity::RTPSEntity,
    endpoint::{EndpointAttributes, Endpoint},
    locator::Locator,
    dds_cache::DDSCache,
  },
  common::timed_event_handler::{TimedEventHandler},
};
use super::{
  qos::{policy, QosPolicies},
  rtps_reader_proxy::RtpsReaderProxy,
  statusevents::*,
};
use policy::{History, Reliability};

#[derive(PartialEq,Eq,Clone,Copy)]
pub enum DeliveryMode {
  Unicast,
  Multicast,
}

pub(crate) struct Writer {
  pub endianness: Endianness,
  pub heartbeat_message_counter: i32,
  /// Configures the mode in which the
  ///Writer operates. If
  ///pushMode==true, then the Writer
  /// will push changes to the reader. If
  /// pushMode==false, changes will
  /// only be announced via heartbeats
  ///and only be sent as response to the
  ///request of a reader
  pub push_mode: bool,
  ///Protocol tuning parameter that
  ///allows the RTPS Writer to
  ///repeatedly announce the
  ///availability of data by sending a
  ///Heartbeat Message.
  pub heartbeat_period: Option<Duration>,
  /// duration to launch cahche change remove from DDSCache
  pub cahce_cleaning_perioid: Duration,
  ///Protocol tuning parameter that
  ///allows the RTPS Writer to delay
  ///the response to a request for data
  ///from a negative acknowledgment.
  pub nack_respose_delay: Duration,
  ///Protocol tuning parameter that
  ///allows the RTPS Writer to ignore
  ///requests for data from negative
  ///acknowledgments that arrive ‘too
  ///soon’ after the corresponding
  ///change is sent.
  pub nack_suppression_duration: Duration,
  ///Internal counter used to assign
  ///increasing sequence number to
  ///each change made by the Writer
  pub last_change_sequence_number: SequenceNumber,
  ///If samples are available in the Writer, identifies the first (lowest)
  ///sequence number that is available in the Writer.
  ///If no samples are available in the Writer, identifies the lowest
  ///sequence number that is yet to be written by the Writer
  pub first_change_sequence_number: SequenceNumber,
  ///Optional attribute that indicates
  ///the maximum size of any
  ///SerializedPayload that may be
  ///sent by the Writer
  pub data_max_size_serialized: u64,
  endpoint_attributes: EndpointAttributes,
  my_guid: GUID,
  pub(crate) writer_command_receiver: mio_channel::Receiver<WriterCommand>,
  ///The RTPS ReaderProxy class represents the information an RTPS StatefulWriter maintains on each matched
  ///RTPS Reader
  readers: BTreeMap<GUID,RtpsReaderProxy>, // TODO: Convert to BTreeMap for faster finds.
  matched_readers_count_total: i32, // all matches, never decremented
  requested_incompatible_qos_count: i32, // how many times a Reader requested incompatible QoS
  message: Option<Message>,
  udp_sender: UDPSender,
  // This writer can read/write to only one of this DDSCache topic caches identified with my_topic_name
  dds_cache: Arc<RwLock<DDSCache>>,
  /// Writer can only read/write to this topic DDSHistoryCache.
  my_topic_name: String,

  /// Maps this writers local sequence numbers to DDSHistodyCache instants.
  /// Useful when negative acknack is recieved.
  sequence_number_to_instant: BTreeMap<SequenceNumber, Timestamp>,

  /// Maps this writers local sequence numbers to DDSHistodyCache instants.
  /// Useful when datawriter dispose is recieved.
  key_to_instant: HashMap<u128, Timestamp>,

  /// Set of disposed samples.
  /// Useful when reader requires some sample with acknack.
  disposed_sequence_numbers: HashSet<SequenceNumber>,

  //When dataWriter sends cacheChange message with cacheKind is NotAlive_Disposed
  //this is set true. If Datawriter after disposing sends new cahceChanges this falg is then
  //turned true.
  //When writer is in disposed state it needs to send StatusInfo_t (PID_STATUS_INFO) with DisposedFlag
  // pub writer_is_disposed: bool,

  ///Contains timer that needs to be set to timeout with duration of self.heartbeat_perioid
  ///timed_event_handler sends notification when timer is up via miochannel to poll in Dp_eventWrapper
  ///this also handles writers cache cleaning timeouts.
  timed_event_handler: Option<TimedEventHandler>,

  qos_policies: QosPolicies,

  // Used for sending status info about messages sent
  status_sender: SyncSender<DataWriterStatus>,
  //offered_deadline_status: OfferedDeadlineMissedStatus,
}

pub(crate) enum WriterCommand {
  DDSData { data: DDSData },
  //ResetOfferedDeadlineMissedStatus { writer_guid: GUID },
}

impl Writer {
  pub fn new(
    guid: GUID,
    writer_command_receiver: mio_channel::Receiver<WriterCommand>,
    dds_cache: Arc<RwLock<DDSCache>>,
    topic_name: String,
    qos_policies: QosPolicies,
    status_sender: SyncSender<DataWriterStatus>,
  ) -> Writer {
    let heartbeat_period = match &qos_policies.reliability {
      Some(r) => match r {
        Reliability::BestEffort => None,
        Reliability::Reliable {
          max_blocking_time: _,
        } => Some(Duration::from_secs(1)),
      },
      None => None,
    };

    let heartbeat_period = match heartbeat_period {
      Some(hbp) => match qos_policies.liveliness {
        Some(lv) => match lv {
          policy::Liveliness::Automatic { lease_duration: _ } => Some(hbp),
          policy::Liveliness::ManualByParticipant { lease_duration: _ } => Some(hbp),
          policy::Liveliness::ManualByTopic { lease_duration } => {
            let std_dur = Duration::from(lease_duration);
            Some(std_dur / 3)
          }
        },
        None => Some(hbp),
      },
      None => None,
    };

    Writer {
      endianness: Endianness::LittleEndian,
      heartbeat_message_counter: 1,
      push_mode: true,
      heartbeat_period,
      cahce_cleaning_perioid: Duration::from_secs(2 * 60),
      nack_respose_delay: Duration::from_millis(200),
      nack_suppression_duration: Duration::from_millis(0),
      last_change_sequence_number: SequenceNumber::from(0),
      first_change_sequence_number: SequenceNumber::from(0),
      data_max_size_serialized: 999999999,
      my_guid: guid,
      //enpoint_attributes: EndpointAttributes::default(),
      writer_command_receiver,
      readers: BTreeMap::new(),
      matched_readers_count_total: 0,
      requested_incompatible_qos_count: 0,
      message: None,
      endpoint_attributes: EndpointAttributes::default(),
      udp_sender: UDPSender::new_with_random_port(),
      dds_cache,
      my_topic_name: topic_name,
      sequence_number_to_instant: BTreeMap::new(),
      key_to_instant: HashMap::new(),
      disposed_sequence_numbers: HashSet::new(),
      timed_event_handler: None,
      qos_policies,
      status_sender,
      //offered_deadline_status: OfferedDeadlineMissedStatus::new(),
    }
  }

  /// To know when token represents a writer we should look entity attribute kind
  /// this entity token can be used in DataWriter -> Writer mio::channel.
  pub fn get_entity_token(&self) -> Token {
    let id = self.get_guid().as_usize();
    Token(id)
  }

  /// This token is used in timed event mio::channel HearbeatHandler -> dpEventwrapper
  pub fn get_timed_event_entity_token(&self) -> Token {
    let mut hasher = DefaultHasher::new();
    let id = self.get_guid().as_usize() as u64;
    hasher.write(&id.to_le_bytes());
    let hashedID: u64 = hasher.finish();
    Token(hashedID as usize)
  }

  pub fn is_reliable(&self) -> bool {
    self.qos_policies.reliability.is_some()
  }

  // TODO:
  // please explain why this is needed and why does it make sense.
  // Used by dp_event_loop.
  pub fn notify_new_data_to_all_readers(&mut self) {
    // removed, because it causes ghost sequence numbers. 
    // for reader_proxy in self.readers.iter_mut() {
    //   reader_proxy.notify_new_cache_change(self.last_change_sequence_number);
    // }
  }

  // --------------------------------------------------------------
  // --------------------------------------------------------------
  // --------------------------------------------------------------

  // Called by dp_wrapper
  pub fn add_timed_event_handler(&mut self, time_handler: TimedEventHandler) {
    self.timed_event_handler = Some(time_handler);
    self.set_cache_cleaning_timer();
    self.set_heartbeat_timer(); // prime the timer
  }


  // --------------------------------------------------------------
  // --------------------------------------------------------------
  // --------------------------------------------------------------

  pub fn handle_timed_event(&mut self, timer_message:TimerMessageType) {
    match timer_message {
      TimerMessageType::WriterHeartbeat => 
        self.handle_heartbeat_tick(false), // false = This is automatic heartbeat by timer, not manual by application call.
      TimerMessageType::WriterCacheCleaning =>
        self.handle_cache_cleaning(),
      TimerMessageType::WriterSendRepairData{ to_reader: r } =>
        self.handle_repair_data_send(r),
      other_msg => 
        error!("handle_timed_event - unexpected message: {:?}", other_msg),
    }
  }

  /// This is called by dp_wrapper everytime cacheCleaning message is received.
  fn handle_cache_cleaning(&mut self) {
    let resource_limit = 32; // TODO: This limit should be obtained
    // from Topic and Writer QoS. There should be some reasonable default limit
    // in case some suppied QoS setting does not specify a larger value.
    // In any case, there has to be some limit to avoid memory leak.

    match self.qos_policies.history {
      None => {
        self.remove_all_acked_changes_but_keep_depth(1);
      }
      Some(History::KeepAll) => {
        self.remove_all_acked_changes_but_keep_depth(resource_limit);
      }
      Some(History::KeepLast { depth: d }) => {
        self.remove_all_acked_changes_but_keep_depth(d as usize);
      }
    }

    self.set_cache_cleaning_timer();
  }

  fn set_cache_cleaning_timer(&mut self) {
    self.timed_event_handler.as_mut().unwrap().set_timeout(
      &chronoDuration::from(self.cahce_cleaning_perioid),
      TimerMessageType::WriterCacheCleaning,
    )
  }

  // --------------------------------------------------------------
  // --------------------------------------------------------------
  // --------------------------------------------------------------

  // Receive new data samples from the DDS DataWriter
  pub fn process_writer_command(&mut self) {
    while let Ok(cc) = self.writer_command_receiver.try_recv() {
      match cc {
        WriterCommand::DDSData { data } => {
          // We have a new sample here. Things to do:
          // 1. Insert it to history cache and get it sequence numbered
          // 2. Send out data. 
          //    If we are pushing data, send the DATA submessage and HEARTBEAT.
          //    If we are not pushing, send out HEARTBEAT only. Readers will then ask the DATA with ACKNACK.
          let timestamp = self.insert_to_history_cache(data);

          self.increase_heartbeat_counter();

          let partial_message = MessageBuilder::new()
            .ts_msg(self.endianness, Some(Timestamp::now()) );
          let data_hb_message_builder = 
            if self.push_mode {
              if let Some(cache_change) = 
                  self.dds_cache.read().unwrap()
                    .from_topic_get_change(&self.my_topic_name, &timestamp) { 
                partial_message.data_msg( cache_change.clone(), // TODO: We should not clone, too much copying
                                          EntityId::ENTITYID_UNKNOWN, // reader
                                          self.my_guid.entityId, // writer
                                          self.endianness ) 
                // TODO: Here we are cloning the entire payload. We need to rewrite the transmit path to avoid copying.
              } else { partial_message }
            } else { partial_message };
          let final_flag = false;
          let liveliness_flag = false;
          let data_hb_message = data_hb_message_builder
               .heartbeat_msg(self, EntityId::ENTITYID_UNKNOWN, final_flag, liveliness_flag)
               .add_header_and_build(self.my_guid.guidPrefix);
          self.send_message_to_readers(DeliveryMode::Multicast, 
            &data_hb_message, &mut self.readers.values() );
        }

        // WriterCommand::ResetOfferedDeadlineMissedStatus { writer_guid: _, } => {
        //   self.reset_offered_deadline_missed_status();
        // }
      }
    }
  }

  fn insert_to_history_cache(&mut self, data: DDSData) -> Timestamp {
    // first increasing last SequenceNumber
    let new_sequence_number = self.last_change_sequence_number + SequenceNumber::from(1);
    self.last_change_sequence_number = new_sequence_number;

    // setting first change sequence number according to our qos (not offering more than our QOS says)
    self.first_change_sequence_number =
      match self.get_qos().history {
        None => self.last_change_sequence_number, // default: depth = 1

        Some(History::KeepAll) =>
          // Now that we have a change, is must be at least one
          max(self.first_change_sequence_number, SequenceNumber::from(1)) ,

        Some(History::KeepLast { depth }) => 
          max( self.last_change_sequence_number - SequenceNumber::from((depth - 1) as i64)
             , SequenceNumber::from(1) ),
      };
    assert!(self.first_change_sequence_number > SequenceNumber::zero() );
    assert!(self.last_change_sequence_number > SequenceNumber::zero() );

    // create new CacheChange from DDSData
    let new_cache_change = CacheChange::new(
      data.change_kind,
      self.get_guid(),
      self.last_change_sequence_number,
      Some(data),
    );
    let data_key = new_cache_change.key;

    // inserting to DDSCache
    let timestamp = Timestamp::now();
    self.dds_cache.write().unwrap().to_topic_add_change(
      &self.my_topic_name,
      &timestamp,
      new_cache_change,
    );

    // keeping table of instant sequence number pairs
    self.sequence_number_to_instant
        .insert(new_sequence_number, timestamp);

    // update key to timestamp mapping   
    self.key_to_instant.insert(data_key, timestamp);

    // Notify reader proxies that there is a new sample
    for reader in &mut self.readers.values_mut() {
      reader.notify_new_cache_change(new_sequence_number)
    }
    timestamp
  }

   // --------------------------------------------------------------
  // --------------------------------------------------------------
  // --------------------------------------------------------------
  
  /// This is called periodically.
  pub fn handle_heartbeat_tick(&mut self, is_manual_assertion: bool ) {
    // Reliable Stateless Writer will set the final flag.
    // Reliable Stateful Writer (that tracks Readers by ReaderProxy) will not set the final flag.
    let final_flag = false;
    let liveliness_flag = is_manual_assertion; // RTPS spec "8.3.7.5 Heartbeat"

    trace!("heartbeat tick in topic {:?} have {} readers", self.topic_name(), self.readers.len());

    self.increase_heartbeat_counter(); 
    // TODO: This produces same heartbeat count for all messages sent, but
    // then again, they represent the same writer status.

    if self.readers.values().all(|rp| self.last_change_sequence_number < rp.all_acked_before ) {
      trace!("heartbeat tick: all readers have all available data.");
    } else {
      let hb_message = MessageBuilder::new()
        .ts_msg(self.endianness, Some(Timestamp::now()) )
        .heartbeat_msg(self, EntityId::ENTITYID_UNKNOWN, final_flag, liveliness_flag)
        .add_header_and_build(self.my_guid.guidPrefix);      
      self.send_message_to_readers(DeliveryMode::Multicast, &hb_message, 
                                    &mut self.readers.values())
    }

    self.set_heartbeat_timer(); // keep the heart beating
  }

  /// after heartbeat is handled timer should be set running again.
  fn set_heartbeat_timer(&mut self) {
    match self.heartbeat_period {
      Some(period) => {
        self.timed_event_handler.as_mut().unwrap().set_timeout(
          &chronoDuration::from(period),
          TimerMessageType::WriterHeartbeat,
        );
        trace!("set heartbeat timer to {:?} in topic {:?}",period, self.topic_name());
      }
      None => (),
    }
  }

  /// When receiving an ACKNACK Message indicating a Reader is missing some data samples, the Writer must
  /// respond by either sending the missing data samples, sending a GAP message when the sample is not relevant, or
  /// sending a HEARTBEAT message when the sample is no longer available
  pub fn handle_ack_nack(&mut self, reader_guid_prefix: GuidPrefix, an: AckNack) {
    // sanity check
    if !self.is_reliable() {
      warn!("Writer {:x?} is best effort! It should not handle acknack messages!", 
              self.get_entity_id());
      return
    }
    // Update the ReaderProxy
    let last_seq = self.last_change_sequence_number; // to avoid borrow problems
    if let Some(reader_proxy) = self.lookup_readerproxy_mut(reader_guid_prefix, an.reader_id) {

        reader_proxy.handle_ack_nack(&an, last_seq);

        let reader_guid = reader_proxy.remote_reader_guid; // copy to avoid double mut borrow 
        // Sanity Check: if the reader asked for something we did not even advertise yet.
        // TODO: This checks the stored unset_changes, not presentely received ACKNACK.
        if let Some(&high) = reader_proxy.unsent_changes.iter().next_back()  {
          if high > last_seq { 
            warn!("ReaderProxy {:?} thinks we need to send {:?} but I have only up to {:?}", 
              reader_proxy.remote_reader_guid, reader_proxy.unsent_changes, last_seq);
          }
        }
        // Sanity Check
        if an.reader_sn_state.base() > last_seq + SequenceNumber::from(1) { // more sanity
          warn!("ACKNACK from {:?} acks up to before {:?} but I have only up to {:?}",
            reader_proxy.remote_reader_guid, reader_proxy.unsent_changes, last_seq);
        }
        // TODO: The following check is rather expensive. Maybe should turn it off
        // in release build?
        if let Some( max_req_sn ) = an.reader_sn_state.iter().max() { // sanity check
          if max_req_sn > last_seq {
            warn!("ACKNACK from {:?} requests {:?} but I have only up to {:?}",
              reader_proxy.remote_reader_guid, 
              an.reader_sn_state.iter().collect::<Vec<SequenceNumber>>(), last_seq);
          }
        }

        // if we cannot send more data, we are done.
        // This is to prevent empty "repair data" messages from being sent.
        if reader_proxy.all_acked_before > last_seq {
          return 
        } else {
          // prime timer to send repair data
          reader_proxy.repair_mode = true; // hold sending normal DATA
          self.timed_event_handler.as_mut().unwrap().set_timeout(
            &chronoDuration::from_std(NACK_RESPONSE_DELAY).unwrap(),
            TimerMessageType::WriterSendRepairData{ to_reader: reader_guid },
          );
        }
    }
  }

  // Send out missing data
  fn handle_repair_data_send(&mut self, to_reader: GUID) {
    if let Some(mut reader_proxy) = self.matched_reader_remove(to_reader) {
      let reader_guid = reader_proxy.remote_reader_guid;
      let mut partial_message = MessageBuilder::new()
        .dst_submessage(self.endianness, reader_guid.guidPrefix)
        .ts_msg(self.endianness, Some(Timestamp::now())); 
        // TODO: This timestamp should probably not be
        // the current (retransmit) time, but the initial sample production timestamp,
        // i.e. should be read from DDSCache (and be implemented there)
      debug!("Repair data send due to ACKNACK. ReaderProxy Unsent changes: {:?}",
              reader_proxy.unsent_changes);

      let mut no_longer_relevant = Vec::new();
      let mut found_data = false;
      if let Some(&unsent_sn) = reader_proxy.unsent_changes.iter().next() {
        match self.sequence_number_to_instant(unsent_sn) {
          Some(timestamp) => {
            if let Some(cache_change) = self.dds_cache.read().unwrap()
                .from_topic_get_change(&self.my_topic_name, &timestamp) {
              partial_message = partial_message
                  .data_msg(cache_change.clone(), // TODO: We should not clone, too much copying
                            reader_guid.entityId, // reader
                            self.my_guid.entityId, // writer
                            self.endianness); 
              // TODO: Here we are cloning the entire payload. We need to rewrite the transmit path to avoid copying.
            } else {
              no_longer_relevant.push(unsent_sn);
            }
          }
          None => {
            error!("handle ack_nack writer {:?} seq.number {:?} missing from instant map", 
                    self.my_guid, unsent_sn);
            no_longer_relevant.push(unsent_sn);
          }  
        } // match
        reader_proxy.unsent_changes.remove(&unsent_sn);
        found_data = true;
      }
      // Add GAP submessage, if some chache changes could not be found.
      if ! no_longer_relevant.is_empty() {
        partial_message = partial_message.gap_msg(BTreeSet::from_iter(no_longer_relevant), &self, reader_guid);
      }
      let data_gap_msg = partial_message
        .add_header_and_build(self.my_guid.guidPrefix);

      self.send_message_to_readers(DeliveryMode::Unicast, &data_gap_msg,
                &mut std::iter::once(&reader_proxy));

      if found_data { // prime repair timer again, if data was found
        // Try to send repait messages at 5x rate compared to usual deadline rate
        let delay_to_next_message = 
          self.qos_policies.deadline().map( |dl| dl.0 )
            .unwrap_or(Duration::from_millis(100)) / 5;
        self.timed_event_handler.as_mut().unwrap().set_timeout(
          &chronoDuration::from(delay_to_next_message),
          TimerMessageType::WriterSendRepairData{ to_reader: reader_proxy.remote_reader_guid },
        );        
      } else {
        reader_proxy.repair_mode = false; // resume normal data sending
      }

      // insert reader back
      self.matched_reader_add(reader_proxy);      
    }   
  } // fn

  /// Removes permanently cacheChanges from DDSCache.
  /// CacheChanges can be safely removed only if they are acked by all readers. (Reliable)
  /// Depth is QoS policy History depth.
  /// Returns SequenceNumbers of removed CacheChanges
  /// This is called repeadedly by handle_cache_cleaning action.
  fn remove_all_acked_changes_but_keep_depth(&mut self, depth: usize)  {
    // All readers have acked up to this point (SequenceNumber)
    let acked_by_all_readers = self.readers.values()
          .map(|r| r.acked_up_to_before())
          .min()
          .unwrap_or(SequenceNumber::zero());
    // If all readers have acked all up to before 5, and depth is 5, we need
    // to keep samples 0..4, i.e. from acked_up_to_before - depth .
    let first_keeper = 
      max( acked_by_all_readers - SequenceNumber::from(depth) , 
            self.first_change_sequence_number );

    // We notify the DDSCache that it can release older samples
    // as far as this Writeris concenrned.
    if let Some(&keep_instant) = self.sequence_number_to_instant.get(&first_keeper) {
      self.dds_cache.write().unwrap()
        .from_topic_remove_before(&self.my_topic_name, keep_instant);
    } else {
      warn!("{:?} missing from instant map",first_keeper);
    }
    self.first_change_sequence_number = first_keeper;
    self.sequence_number_to_instant = 
      self.sequence_number_to_instant.split_off(&first_keeper);
  }

  fn increase_heartbeat_counter(&mut self) {
    self.heartbeat_message_counter = self.heartbeat_message_counter + 1;
  }

  fn send_message_to_readers(&self, preferred_mode: DeliveryMode, message: &Message, 
        readers: &mut dyn Iterator<Item = &RtpsReaderProxy>) {
    // TODO: This is a stupid transmit algorithm. We should compute a preferred
    // unicast and multicast locators for each reader only on every reader update, and
    // not find it dynamically on every message.
    let buffer = message.write_to_vec_with_ctx(self.endianness).unwrap();
    let mut already_sent_to = BTreeSet::new();

    macro_rules! send_unless_sent_and_mark {
      ($loc:expr) => {
        if already_sent_to.contains($loc) {
          trace!("Already sent to {:?}", $loc);
        } else {
          self.udp_sender.send_to_locator(&buffer, $loc);
          already_sent_to.insert($loc.clone());
        }
      }
    }

    for reader in readers {
      match ( preferred_mode, 
              reader.unicast_locator_list.iter().find(|l| Locator::isUDP(l) ), 
              reader.multicast_locator_list.iter().find(|l| Locator::isUDP(l) ) ) {
        (DeliveryMode::Multicast, _ , Some(mc_locator)) => {
          send_unless_sent_and_mark!(mc_locator)
        }
        (DeliveryMode::Unicast, Some(uc_locator) , _ ) => {
          send_unless_sent_and_mark!(uc_locator)
        }        
        (_delivery_mode, _ , Some(mc_locator)) => {
          send_unless_sent_and_mark!(mc_locator)
        }
        (_delivery_mode, Some(uc_locator), _ ) => {
          send_unless_sent_and_mark!(uc_locator)
        }
        (_delivery_mode, None, None ) => {
          error!("send_message_to_readers: No locators for {:?}",reader);
        }
      } // match
    }

  }
 
  // returns if the reader was new
  pub fn update_reader_proxy(&mut self, reader_proxy: RtpsReaderProxy, requested_qos:QosPolicies) {
    if self.readers.get( &reader_proxy.remote_reader_guid ).is_none() {
      self.readers.insert( reader_proxy.remote_reader_guid , reader_proxy );
    } else {
      match  self.qos_policies.complies_to_fail(&requested_qos) {
        // matched QoS
        None => {
          self.readers.insert( reader_proxy.remote_reader_guid , reader_proxy );
          self.matched_readers_count_total += 1;
          self.status_sender.try_send(DataWriterStatus::PublicationMatched { 
                total: CountWithChange::new(self.matched_readers_count_total , 1 ),
                current: CountWithChange::new(self.readers.len() as i32 , 1)
              })
            .unwrap_or_else(|send_err| error!("status send error: {:?}", send_err));
          // send out hearbeat, so that new reader can catch up
          if self.qos_policies.reliability.is_some() {
            self.notify_new_data_to_all_readers()
          }
        }
        Some(bad_policy_id) => {
          // QoS not compliant :(
          self.requested_incompatible_qos_count += 1;
          self.status_sender.try_send(DataWriterStatus::OfferedIncompatibleQos { 
                count: CountWithChange::new(self.requested_incompatible_qos_count , 1 ),
                last_policy_id: bad_policy_id,
                policies: Vec::new(), // TODO: implement this
              })
            .unwrap_or_else(|send_err| error!("status send error: {:?}", send_err));
        }
      } // match
    } // if 
  }

  // internal helper. Same as above, but duplicaes are an error.
  fn matched_reader_add(&mut self, reader_proxy: RtpsReaderProxy) {
    let guid = reader_proxy.remote_reader_guid;
    if self.readers.insert(reader_proxy.remote_reader_guid, reader_proxy).is_some() {
      error!("Reader proxy with same GUID {:?} added already", guid);
    }
  }

  fn matched_reader_remove(&mut self, guid: GUID,) -> Option<RtpsReaderProxy> {
    self.readers.remove(&guid)
  }

  pub fn reader_lost(&mut self, guid: GUID)
  {
    if self.readers.contains_key(&guid) {
      self.matched_reader_remove(guid);
      //self.matched_readers_count_total -= 1; // this never decreases
      self.status_sender.try_send(DataWriterStatus::PublicationMatched { 
                total: CountWithChange::new(self.matched_readers_count_total , 0 ),
                current: CountWithChange::new(self.readers.len() as i32 , -1)
              })
            .unwrap_or_else(|send_err| error!("status send error: {:?}", send_err));
    }
  }

  ///This operation finds the ReaderProxy with GUID_t a_reader_guid from the set
  /// get guid Prefix from RTPS message main header
  /// get reader guid from AckNack submessage readerEntityId
  fn lookup_readerproxy_mut(
    &mut self,
    guid_prefix: GuidPrefix,
    reader_entity_id: EntityId,
  ) -> Option<&mut RtpsReaderProxy> {
    // TODO: This lookup is done very frequently. It should be done by some
    // map instead of iterating through a Vec.
      let search_guid: GUID = GUID::new_with_prefix_and_id(guid_prefix, reader_entity_id);
    self.readers.get_mut(&search_guid)
  }

  pub fn sequence_number_to_instant(&self, seqnumber: SequenceNumber) -> Option<Timestamp> {
    self.sequence_number_to_instant.get(&seqnumber).copied()
  }

  pub fn find_cache_change(&self, instant: &Timestamp) -> Option<CacheChange> {
    match self.dds_cache.read() {
      Ok(dc) => {
        let cc = dc.from_topic_get_change(&self.my_topic_name, instant);
        cc.cloned()
      }
      Err(e) => panic!("DDSCache is poisoned {:?}", e),
    }
  }

  pub fn topic_name(&self) -> &String {
    &self.my_topic_name
  }

  // pub fn reset_offered_deadline_missed_status(&mut self) {
  //   self.offered_deadline_status.reset_change();
  // }
}

impl RTPSEntity for Writer {
  fn get_guid(&self) -> GUID {
    self.my_guid
  }
}

impl Endpoint for Writer {
  fn as_endpoint(&self) -> &crate::structure::endpoint::EndpointAttributes {
    &self.endpoint_attributes
  }
}

impl HasQoSPolicy for Writer {
  fn get_qos(&self) -> QosPolicies {
    self.qos_policies.clone()
  }
}

// -------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crate::{
    dds::{
      participant::DomainParticipant, qos::QosPolicies, with_key::datawriter::DataWriter,
      topic::TopicKind,
    },
  };
  use std::thread;
  use crate::test::random_data::*;
  use crate::serialization::cdr_serializer::CDRSerializerAdapter;
  use byteorder::LittleEndian;
  use log::info;

  #[test]
  fn test_writer_recieves_datawriter_cache_change_notifications() {
    let domain_participant = DomainParticipant::new(0);
    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();

    let publisher = domain_participant.unwrap()
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant.unwrap()
      .create_topic("Aasii", "Huh?", &qos, TopicKind::WithKey)
      .expect("Failed to create topic");
    let data_writer: DataWriter<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter(None, topic, None)
        .expect("Failed to create datawriter");

    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    let data2 = RandomData {
      a: 2,
      b: "Fobar".to_string(),
    };

    let data3 = RandomData {
      a: 3,
      b: "Fobar".to_string(),
    };

    let writeResult = data_writer.write(data, None).expect("Unable to write data");

    info!("writerResult:  {:?}", writeResult);
    let writeResult = data_writer
      .write(data2, None)
      .expect("Unable to write data");

    info!("writerResult:  {:?}", writeResult);
    let writeResult = data_writer
      .write(data3, None)
      .expect("Unable to write data");

    thread::sleep(std::time::Duration::from_millis(100));
    info!("writerResult:  {:?}", writeResult);
  }
}
