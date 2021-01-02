
use chrono::Duration as chronoDuration;
use enumflags2::BitFlags;

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
  messages::submessages::{
    submessage::EntitySubmessage,
    info_timestamp::InfoTimestamp,
    submessage_elements::{parameter::Parameter, parameter_list::ParameterList},
    submessage_elements::serialized_payload::RepresentationIdentifier,
    submessage_flag::*,
  },
  structure::parameter_id::ParameterId,
};
use crate::messages::submessages::data::Data;
use crate::structure::time::Timestamp;
use crate::structure::duration::Duration;
use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::{header::Header, vendor_id::VendorId, protocol_id::ProtocolId};
use crate::structure::guid::{GuidPrefix, EntityId, EntityKind, GUID};
use crate::structure::sequence_number::{SequenceNumber};
use crate::{
  messages::submessages::submessages::{
    SubmessageHeader, SubmessageKind, InterpreterSubmessage, AckNack, InfoDestination,
  },
  structure::cache_change::{CacheChange, ChangeKind},
  serialization::{SubMessage, Message, SubmessageBody},
};

use crate::dds::{ddsdata::DDSData, qos::HasQoSPolicy};
use crate::{
  network::{constant::TimerMessageType, udp_sender::UDPSender},
  structure::{
    entity::RTPSEntity,
    endpoint::{EndpointAttributes, Endpoint},
    locator::LocatorKind,locator::Locator,
    dds_cache::DDSCache,
  },
  common::timed_event_handler::{TimedEventHandler},
};
use super::{
  qos::{policy, QosPolicies},
  rtps_reader_proxy::RtpsReaderProxy,
  values::result::OfferedDeadlineMissedStatus,
  values::result::StatusChange,
};
use policy::{History, Reliability};

#[derive(PartialEq,Eq,Clone,Copy)]
pub enum DeliveryMode {
  Unicast,
  Multicast,
}

pub(crate) struct Writer {
  source_version: ProtocolVersion,
  source_vendor_id: VendorId,
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
  pub readers: Vec<RtpsReaderProxy>,
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
  status_sender: SyncSender<StatusChange>,
  offered_deadline_status: OfferedDeadlineMissedStatus,
}

pub(crate) enum WriterCommand {
  DDSData { data: DDSData },
  ResetOfferedDeadlineMissedStatus { writer_guid: GUID },
}

impl Writer {
  pub fn new(
    guid: GUID,
    writer_command_receiver: mio_channel::Receiver<WriterCommand>,
    dds_cache: Arc<RwLock<DDSCache>>,
    topic_name: String,
    qos_policies: QosPolicies,
    status_sender: SyncSender<StatusChange>,
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
      source_version: ProtocolVersion::PROTOCOLVERSION_2_3,
      source_vendor_id: VendorId::THIS_IMPLEMENTATION,
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
      readers: vec![],
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
      offered_deadline_status: OfferedDeadlineMissedStatus::new(),
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
  // Used by dp_event_wrapper.
  pub fn notify_new_data_to_all_readers(&mut self) {
    for reader_proxy in self.readers.iter_mut() {
      reader_proxy.notify_new_cache_change(
        max(self.last_change_sequence_number, SequenceNumber::default()));
    }
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

  /// This is called by dp_wrapper everytime cacheCleaning message is received.
  pub fn handle_cache_cleaning(&mut self) {
    let mut removedChanges = vec![];
    match self.qos_policies.history {
      None => {
        removedChanges = self.remove_all_acked_changes_but_keep_depth(0);
      }
      Some(History::KeepAll) => {
        // TODO need to find a way to check if resource limit.
        // writer can only know the amount its own samples.
        //todo!("TODO keep all resource limit ???");
      }
      Some(History::KeepLast { depth: d }) => {
        removedChanges = self.remove_all_acked_changes_but_keep_depth(d);
      }
    }
    // This is needdd to be removed also if cahceChange is removed from DDSCache.
    for sq in removedChanges {
      self.sequence_number_to_instant.remove(&sq);
    }
    self.set_cache_cleaning_timer();
  }

  fn set_cache_cleaning_timer(&mut self) {
    self.timed_event_handler.as_mut().unwrap().set_timeout(
      &chronoDuration::from(self.cahce_cleaning_perioid),
      TimerMessageType::writer_cache_cleaning,
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
            .ts_msg(self.endianness, false);
          let data_hb_message_builder = 
            if self.push_mode {
              if let Some(cache_change) = 
                  self.dds_cache.read().unwrap()
                    .from_topic_get_change(&self.my_topic_name, &timestamp) { 
                partial_message.data_msg(cache_change.clone(), &self, EntityId::ENTITYID_UNKNOWN ) 
                // TODO: Here we are cloning the entire payload. We need to rewrite the transmit path to avoid copying.
              } else { partial_message }
            } else { partial_message };
          let final_flag = false;
          let liveliness_flag = false;
          let data_hb_message = data_hb_message_builder
               .heartbeat_msg(self, EntityId::ENTITYID_UNKNOWN, final_flag, liveliness_flag)
               .add_header_and_build(self.create_message_header());
          self.send_message_to_readers(DeliveryMode::Multicast, 
            &data_hb_message, &mut self.readers.iter() );
        }

        WriterCommand::ResetOfferedDeadlineMissedStatus { writer_guid: _, } => {
          self.reset_offered_deadline_missed_status();
        }
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
    for reader in &mut self.readers {
      reader.notify_new_cache_change(new_sequence_number)
    }
    timestamp
  }

   // --------------------------------------------------------------
  // --------------------------------------------------------------
  // --------------------------------------------------------------
  
  /// This is called periodically.
  pub fn handle_heartbeat_tick(&mut self, is_manual_assertion: bool ) {
    // TODO Set some guidprefix if needed at all.
    // Not sure if DST submessage and TS submessage are needed when sending heartbeat.

    // Reliable Stateless Writer will set the final flag.
    // Reliable Stateful Writer (that tracks Readers by ReaderProxy) will not set the final flag.
    let final_flag = false;
    let liveliness_flag = is_manual_assertion; // RTPS spec "8.3.7.5 Heartbeat"

    trace!("heartbeat tick in topic {:?} have {} readers", self.topic_name(), self.readers.len());

    self.increase_heartbeat_counter(); 
    // TODO: This produces same heartbeat count for all messages sent, but
    // then again, they represent the same writer status.

    for reader in self.readers.iter() {
      // Do we have some changes this reader has not ACKed yet?
      if self.last_change_sequence_number >= reader.all_acked_before {
        // Yes, send heartbeat messages
        let reader_guid = reader.remote_reader_guid;
        let hb_message = MessageBuilder::new()
          .dst_submessage(self.endianness, reader_guid.guidPrefix)
          .ts_msg(self.endianness, false)
          .heartbeat_msg(self, reader_guid.entityId, final_flag, liveliness_flag)
          .add_header_and_build(self.create_message_header());
        self.send_unicast_message_to_reader(&hb_message, reader);
        self.send_multicast_message_to_reader(&hb_message, reader);
      }
    }

    self.set_heartbeat_timer(); // keep the heart beating
  }

  /// after heartbeat is handled timer should be set running again.
  fn set_heartbeat_timer(&mut self) {
    match self.heartbeat_period {
      Some(period) => {
        self.timed_event_handler.as_mut().unwrap().set_timeout(
          &chronoDuration::from(period),
          TimerMessageType::writer_heartbeat,
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
        reader_proxy.handle_ack_nack(&an);

        // if we cannot send more data, we are done.
        // This is to prevent empty "missing data" messages from being sent.
        if reader_proxy.all_acked_before > last_seq {
          // Sanity Check: if the reader asked for something we did not even advertise yet.
          // TODO: This checks the stored unset_changes, not presentely received ACKNACK.
          if let Some(&high) = reader_proxy.unsent_changes.range(..).next_back()  {
            if high > last_seq { 
              info!("ReaderProxy {:?} asked for {:?} but I have only up to {:?}", 
                reader_proxy.remote_reader_guid, reader_proxy.unsent_changes, last_seq);
            }
          }
          return 
        }
    }

    // Send out missing data
    // TODO: We should implement this in a timed action, because RTPS spec says to wait for
    // nack_response_delay before sending out data, and the default delay is not zero.  
    if let Some(reader_proxy) = self
      .matched_reader_remove(GUID::new_with_prefix_and_id(reader_guid_prefix, an.reader_id)) {
        let reader_guid = reader_proxy.remote_reader_guid;
        let mut partial_message = MessageBuilder::new()
          .dst_submessage(self.endianness, reader_guid.guidPrefix)
          .ts_msg(self.endianness, false);
        debug!("Repair data send due to ACKNACK = {:?}\nACKNACK SN Set: {:?}\nReaderProxy Unsent changes: {:?}",
                an, 
                an.reader_sn_state.iter().collect::<Vec<SequenceNumber>>(),
                reader_proxy.unsent_changes);

        let mut no_longer_relevant = Vec::new();
        for &unsent_sn in reader_proxy.unsent_changes.iter() {
          match self.sequence_number_to_instant(unsent_sn) {
            Some(timestamp) => {
              if let Some(cache_change) = self.dds_cache.read().unwrap()
                  .from_topic_get_change(&self.my_topic_name, &timestamp) {
                // TODO: We should not just append DATA submessages blindly. 
                // We should check that message size limit is not exceeded.
                partial_message = partial_message.data_msg(cache_change.clone(), 
                    &self, reader_guid.entityId ); 
                // TODO: Here we are cloning the entire payload. We need to rewrite the transmit path to avoid copying.
              } else {
                no_longer_relevant.push(unsent_sn);
              }
            }
            None => 
              error!("handle ack_nack writer {:?} seq.number {:?} missing from instant map", 
                      self.my_guid, unsent_sn),
          }
        }
        // Add GAP submessage, if some chache changes could not be found.
        if no_longer_relevant.len() > 0 {
          partial_message = partial_message.gap_msg(BTreeSet::from_iter(no_longer_relevant), &self, reader_guid);
        }
        let data_gap_msg = partial_message
          .add_header_and_build(self.create_message_header());
        // TODO: Do we really need to send multicast also?
        // At least we should have one call to send the messages out.
        self.send_unicast_message_to_reader(&data_gap_msg, &reader_proxy);
        self.send_multicast_message_to_reader(&data_gap_msg, &reader_proxy);

        // insert reader back
        self.matched_reader_add(reader_proxy);
      }
      
  } // fn

  /// Removes permanently cacheChanges from DDSCache.
  /// CacheChanges can be safely removed only if they are acked by all readers. (Reliable)
  /// Depth is QoS policy History depth.
  /// Returns SequenceNumbers of removed CacheChanges
  /// This is called repeadedly by handle_cache_cleaning action.
  fn remove_all_acked_changes_but_keep_depth(&mut self, depth: i32) -> Vec<SequenceNumber> {
    let amount_need_to_remove;
    let mut removed_change_sequence_numbers = vec![];
    let acked_by_all_readers = {
      //let mut acked_by_all: Vec<(&Timestamp, &SequenceNumber)> = vec![];
      let mut acked_by_all: BTreeMap<&Timestamp, &SequenceNumber> = BTreeMap::new();
      for (sq, i) in self.sequence_number_to_instant.iter() {
        if self.change_with_sequence_number_is_acked_by_all(*sq) {
          acked_by_all.insert(i, sq);
        }
      }
      acked_by_all
    };
    if acked_by_all_readers.len() as i32 <= depth {
      return removed_change_sequence_numbers;
    } else {
      amount_need_to_remove = acked_by_all_readers.len() as i32 - depth;
    }
    {
      let mut index: i32 = 0;
      for (i, _sq) in acked_by_all_readers {
        if index >= amount_need_to_remove {
          break;
        }
        let removed = self
          .dds_cache
          .write()
          .unwrap()
          .from_topic_remove_change(&self.my_topic_name, i);
        if removed.is_some() {
          removed_change_sequence_numbers.push(removed.unwrap().sequence_number);
        } else {
          warn!(
            "Remove from {:?} failed. No results with {:?}",
            &self.my_topic_name, i
          );
          debug!("{:?}", self.dds_cache);
        }
        index = index + 1;
      }
    }
    return removed_change_sequence_numbers;
  }

  fn remove_from_history_cache(&mut self, instant: &Timestamp) {
    let removed_change = self
      .dds_cache
      .write()
      .unwrap()
      .from_topic_remove_change(&self.my_topic_name, instant);
    debug!("removed change from DDShistoryCache {:?}", removed_change);
    if removed_change.is_some() {
      self
        .disposed_sequence_numbers
        .insert(removed_change.unwrap().sequence_number);
    } else {
      todo!();
    }
  }

  fn remove_from_history_cache_with_sequence_number(&mut self, sequence_number: &SequenceNumber) {
    let instant = self.sequence_number_to_instant.get(sequence_number);
    if instant.is_some() {
      let removed_change = self
        .dds_cache
        .write()
        .unwrap()
        .from_topic_remove_change(&self.my_topic_name, instant.unwrap());
      if removed_change.is_none() {
        todo!(
          "Cache change with seqnum {:?} and instant {:?} cold not be revod from DDSCache",
          sequence_number,
          instant
        )
      }
    } else {
      todo!(
        "sequence number: {:?} cannot be tranformed to instant ",
        sequence_number
      );
    }
  }

   fn increase_heartbeat_counter(&mut self) {
    self.heartbeat_message_counter = self.heartbeat_message_counter + 1;
  }


  fn send_unicast_message_to_reader(&self, message: &Message, reader: &RtpsReaderProxy) {
    if let Ok(data) = message.write_to_vec_with_ctx(self.endianness) {
      self
        .udp_sender
        .send_to_locator_list(&data, &reader.unicast_locator_list)
    }
  }

  fn send_multicast_message_to_reader(&self, message: &Message, reader: &RtpsReaderProxy) {
    let buffer = message.write_to_vec_with_ctx(self.endianness).unwrap();
    for multiaddress in &reader.multicast_locator_list {
      if multiaddress.kind == LocatorKind::LOCATOR_KIND_UDPv4 {
        self
          .udp_sender
          .send_ipv4_multicast(&buffer, multiaddress.to_socket_address())
          .expect("Unable to send multicast message.");
      } else if multiaddress.kind == LocatorKind::LOCATOR_KIND_UDPv6 {
        todo!();
      }
    }
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

  fn create_message_header(&self) -> Header {
    Header {
      protocol_id: ProtocolId::default(),
      protocol_version: ProtocolVersion {
        major: self.source_version.major,
        minor: self.source_version.minor,
      },
      vendor_id: VendorId {
        vendorId: self.source_vendor_id.vendorId,
      },
      guid_prefix: self.my_guid.guidPrefix,
    }
  }

  // TODO: Is this copy-paste code from serialization/message.rs
  pub fn get_TS_submessage(&self, invalidiateFlagSet: bool) -> SubMessage {
    let timestamp = InfoTimestamp {
      timestamp: Timestamp::now(),
    };
    let mes = &mut timestamp.write_to_vec_with_ctx(self.endianness).unwrap();

    let flags = BitFlags::<INFOTIMESTAMP_Flags>::from_endianness(self.endianness)
      | (if invalidiateFlagSet {
        INFOTIMESTAMP_Flags::Invalidate.into()
      } else {
        BitFlags::<INFOTIMESTAMP_Flags>::empty()
      });

    let submessageHeader = SubmessageHeader {
      kind: SubmessageKind::INFO_TS,
      flags: flags.bits(),
      content_length: mes.len() as u16, // This conversion should be safe, as timestamp length cannot exceed u16
    };
    SubMessage {
      header: submessageHeader,
      body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoTimestamp(timestamp, flags)),
    }
  }

  // TODO: Is this copy-paste code from serialization/message.rs
  pub fn get_DST_submessage(endianness: Endianness, guid_prefix: GuidPrefix) -> SubMessage {
    let flags = BitFlags::<INFODESTINATION_Flags>::from_endianness(endianness);
    let submessageHeader = SubmessageHeader {
      kind: SubmessageKind::INFO_DST,
      flags: flags.bits(),
      content_length: 12u16,
      //InfoDST length is always 12 because message contains only GuidPrefix
    };
    SubMessage {
      header: submessageHeader,
      body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoDestination(
        InfoDestination { guid_prefix },
        flags,
      )),
    }
  }


  // Called by message.rs to construct DATA Submessage.
  // TODO: This code should really moved there to break the dependency in the wrong direction.
  pub fn get_DATA_msg_from_cache_change(
    &self,
    change: CacheChange,
    reader_entity_id: EntityId,
  ) -> SubMessage {

    let inline_qos = match change.kind {
      ChangeKind::ALIVE => None,
      _ => {
        let mut param_list = ParameterList::new();
        let key_hash = Parameter {
          parameter_id: ParameterId::PID_KEY_HASH,
          value: change.key.to_le_bytes().to_vec(),
        };
        param_list.parameters.push(key_hash);
        let status_info = Parameter::create_pid_status_info_parameter(true, true, false);
        param_list.parameters.push(status_info);
        Some(param_list)
      }
    };

    let mut data_message = Data {
      reader_id: reader_entity_id,
      writer_id: self.get_entity_id(), // TODO! Is this the correct EntityId here?
      writer_sn: change.sequence_number,
      inline_qos,
      serialized_payload: change.data_value,
    };
    
    // TODO: please explain this logic here:
    if self.get_entity_id().get_kind() == EntityKind::WRITER_WITH_KEY_BUILT_IN {
      match data_message.serialized_payload.as_mut() {
        Some(sp) => sp.representation_identifier = RepresentationIdentifier::PL_CDR_LE,
        None => (),
      }
    }

    let flags: BitFlags<DATA_Flags> = BitFlags::<DATA_Flags>::from_endianness(self.endianness)
      | (
        if change.kind == ChangeKind::NOT_ALIVE_DISPOSED {
          // No data, we send key instead
          BitFlags::<DATA_Flags>::from_flag(DATA_Flags::InlineQos)
        } else {
          BitFlags::<DATA_Flags>::from_flag(DATA_Flags::Data)
        }
        // normal case
      );

    let size = data_message
      .write_to_vec_with_ctx(self.endianness)
      .unwrap()
      .len() as u16;

    SubMessage {
      header: SubmessageHeader {
        kind: SubmessageKind::DATA,
        flags: flags.bits(),
        content_length: size,
      },
      body: SubmessageBody::Entity(EntitySubmessage::Data(data_message, flags)),
    }
  }

  pub fn matched_reader_add(&mut self, reader_proxy: RtpsReaderProxy) {
    if self.readers.iter().any(|x| {
      x.remote_group_entity_id == reader_proxy.remote_group_entity_id
        && x.remote_reader_guid == reader_proxy.remote_reader_guid
    }) {
      panic!("Reader proxy with same group entityid and remotereader guid added already");
    };
    &self.readers.push(reader_proxy);
  }

  pub fn matched_reader_remove(&mut self, guid: GUID,) -> Option<RtpsReaderProxy> {
    let pos = self.readers.iter().position(|x| x.remote_reader_guid == guid );
    pos.map( |p| self.readers.remove(p) )
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
    self
      .readers
      .iter_mut()
      .find(|x| x.remote_reader_guid == search_guid)
  }

  pub fn change_with_sequence_number_is_acked_by_all(
    &self,
    sequence_number: SequenceNumber,
  ) -> bool {
    self.readers.iter().all( |rp| rp.sequence_is_acked(sequence_number) )
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

  pub fn reset_offered_deadline_missed_status(&mut self) {
    self.offered_deadline_status.reset_change();
  }
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

    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
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
