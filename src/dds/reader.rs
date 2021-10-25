use crate::{
  //common::timed_event_handler::TimedEventHandler,
  //network::constant::TimerMessageType,
  structure::{cache_change::ChangeKind, entity::RTPSEntity},
};
use crate::structure::endpoint::{Endpoint, EndpointAttributes};
use crate::messages::submessages::submessages::*;

use crate::dds::ddsdata::DDSData;
use crate::dds::statusevents::*;
use crate::dds::rtps_writer_proxy::RtpsWriterProxy;
use crate::structure::guid::{GUID, EntityId, GuidPrefix};
use crate::structure::sequence_number::{SequenceNumber, SequenceNumberSet};
#[cfg(test)] use crate::structure::locator::LocatorList;
use crate::structure::{duration::Duration, time::Timestamp};

use std::{
  collections::BTreeSet,
  iter::FromIterator,
  sync::{Arc, RwLock},
  rc::Rc,
};
use crate::structure::dds_cache::{DDSCache};
//use std::time::Instant;

use mio::Token;
use mio_extras::timer::Timer;
use mio_extras::channel as mio_channel;
use log::{debug, info, warn, trace, error};
use std::fmt;

use std::collections::{BTreeMap};
use std::time::Duration as StdDuration;
use enumflags2::BitFlags;

use crate::structure::cache_change::CacheChange;
use crate::dds::message_receiver::MessageReceiverState;
use crate::dds::qos::{QosPolicies, HasQoSPolicy, policy};
use crate::network::udp_sender::UDPSender;

use crate::serialization::message::Message;
use crate::messages::header::Header;
use crate::messages::protocol_id::ProtocolId;
use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::vendor_id::VendorId;
use crate::messages::submessages::submessage_elements::parameter_list::ParameterList;

use speedy::{Writable, Endianness};
//use chrono::Duration as chronoDuration;

use super::{
  with_key::datareader::ReaderCommand,
};

use super::qos::InlineQos;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimedEvent {
  DeadlineMissedCheck,
}

// Some pieces necessary to contruct a reader.
// These can be sent between threads, whereas a Reader cannot.
pub (crate) struct ReaderIngredients {
  pub guid: GUID,
  pub notification_sender: mio_channel::SyncSender<()>,
  pub status_sender: mio_channel::SyncSender<DataReaderStatus>,
  pub topic_name: String,
  pub qos_policy: QosPolicies,
  pub data_reader_command_receiver: mio_channel::Receiver<ReaderCommand>, 
}

impl fmt::Debug for ReaderIngredients {
  // Need manual implementation, because channels cannot be Dubug formatted.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Reader")
      .field("my_guid", &self.guid)
      .field("topic_name", &self.topic_name)
      .field("qos_policy", &self.qos_policy)
      .finish()
  }
}

pub(crate) struct Reader {
  // Should the instant be sent?
  notification_sender: mio_channel::SyncSender<()>,
  status_sender: mio_channel::SyncSender<DataReaderStatus>,
  udp_sender: Rc<UDPSender>,

  is_stateful: bool, // is this StatefulReader or Statelessreader as per RTPS spec
  // Currently we support only stateful behaviour.

  dds_cache: Arc<RwLock<DDSCache>>,

  #[cfg(test)] seqnum_instant_map: BTreeMap<SequenceNumber, Timestamp>,

  topic_name: String,
  qos_policy: QosPolicies,

  my_guid: GUID,
  pub enpoint_attributes: EndpointAttributes,

  heartbeat_response_delay: StdDuration,
  heartbeat_supression_duration: StdDuration,

  sent_ack_nack_count: i32,
  received_hearbeat_count: i32,

  matched_writers: BTreeMap<GUID, RtpsWriterProxy>,
  writer_match_count_total: i32, // total count, never decreases

  requested_deadline_missed_count: i32,
  offered_incompatible_qos_count: i32,

  //timed_event_handler: Option<TimedEventHandler>,
  pub(crate) timed_event_timer: Timer<TimedEvent>,
  pub(crate) data_reader_command_receiver: mio_channel::Receiver<ReaderCommand>,

} 

impl Reader {
  pub fn new(
    i : ReaderIngredients,
    dds_cache: Arc<RwLock<DDSCache>>,
    udp_sender: Rc<UDPSender>,
    timed_event_timer: Timer<TimedEvent>,
  ) -> Reader {
      Reader {
        notification_sender: i.notification_sender,
        status_sender: i.status_sender,
        udp_sender,
        is_stateful: true, // Do not change this before stateless functionality is implemented.
        dds_cache,
        topic_name: i.topic_name,
        qos_policy: i.qos_policy,


      #[cfg(test)]
      seqnum_instant_map: BTreeMap::new(),
      my_guid: i.guid ,
      enpoint_attributes: EndpointAttributes::default(),

      heartbeat_response_delay: StdDuration::new(0, 500_000_000), // 0,5sec
      heartbeat_supression_duration: StdDuration::new(0, 0),
      sent_ack_nack_count: 0,
      received_hearbeat_count: 0,
      matched_writers: BTreeMap::new(),
      writer_match_count_total: 0,
      requested_deadline_missed_count: 0,
      offered_incompatible_qos_count: 0,
      timed_event_timer,
      data_reader_command_receiver: i.data_reader_command_receiver,
    }
  }
  // TODO: check if it's necessary to implement different handlers for discovery
  // and user messages

  // TODO Used for test/debugging purposes
  #[cfg(test)]
  pub fn get_history_cache_change_data(&self, sequence_number: SequenceNumber) -> Option<DDSData> {
    let dds_cache = self.dds_cache.read().unwrap();
    let cc = dds_cache.from_topic_get_change(
      &self.topic_name,
      &self.seqnum_instant_map.get(&sequence_number).unwrap(),
    );

    debug!("history cache !!!! {:?}", cc);

    match cc {
      Some(cc) => Some(cc.data_value.clone()) , //  Some(DDSData::new(cc.data_value.as_ref().unwrap().clone())),
      None => None,
    }
  }

  /// To know when token represents a reader we should look entity attribute kind
  pub fn get_entity_token(&self) -> Token {
    self.get_guid().entityId.as_token()
  }

  pub fn get_reader_alt_entity_token(&self) -> Token {
    self.get_guid().entityId.as_alt_token()
  }


  pub fn set_requested_deadline_check_timer(&mut self) {
    if let Some(deadline) = self.qos_policy.deadline {
      debug!("GUID={:?} set_requested_deadline_check_timer: {:?}", self.my_guid, deadline.0.to_std() );
      self.timed_event_timer
        .set_timeout(deadline.0.to_std(), TimedEvent::DeadlineMissedCheck ); 
    } else {
      trace!("GUID={:?} - no deaadline policy - do not set set_requested_deadline_check_timer",
        self.my_guid);
    }
  }

  pub fn send_status_change(&self, change: DataReaderStatus) {
    match self.status_sender.try_send(change) {
      Ok(()) => (), // expected result
      Err(mio_channel::TrySendError::Full(_)) => {
        trace!("Reader cannot send new status changes, datareader is full.");
        // It is perfectly normal to fail due to full channel, because
        // no-one is required to be listening to these.
      }
      Err(mio_channel::TrySendError::Disconnected(_)) => {
        // If we get here, our DataReader has died. The Reader should now dispose itself.
        // Or possibly it has lost the receiver object, which is sort of sloppy,
        // but does not necessarily mean the end of the world.
        // TODO: Implement Reader disposal.
        info!("send_status_change - cannot send status, DataReader Disconnected.")
      }
      Err(mio_channel::TrySendError::Io(e)) => {
        error!("send_status_change - cannot send status: {:?}",e);
      }
    }
  }

  // The deadline that the DataReader was expecting through its QosPolicy
  // DEADLINE was not respected for a specific instance
  // if statusChange is returned it should be send to DataReader
  // this calculation should be repeated every self.qos_policy.deadline
  fn calculate_if_requested_deadline_is_missed(&mut self) -> Vec<DataReaderStatus> {
    debug!("calculate_if_requested_deadline_is_missed");
    
    match self.qos_policy.deadline {
      None => vec![],
      
      Some(policy::Deadline(deadline_duration)) => {
        let mut changes: Vec<DataReaderStatus> = vec![];
        let now = Timestamp::now();
        for (_g, writer_proxy) in self.matched_writers.iter_mut() {
          match writer_proxy.last_change_timestamp() {
            Some(last_change) => {
              let since_last = now.duration_since(last_change);
              // if time singe last received message is greater than deadline increase status and return notification.
              trace!("Comparing deadlines: {:?} - {:?}", since_last, deadline_duration);
              if since_last > deadline_duration{
                debug!("Deadline missed: {:?} - {:?}", since_last, deadline_duration);
                self.requested_deadline_missed_count += 1;
                changes.push( DataReaderStatus::RequestedDeadlineMissed {
                                count: CountWithChange::start_from(self.requested_deadline_missed_count,1)
                              } 
                );
              }      
            }
            None => {
              // no messages recieved ever so deadline must be missed.
              // TODO: But what if the Reader or WriterProxy was just created?
              self.requested_deadline_missed_count += 1;
              changes.push( DataReaderStatus::RequestedDeadlineMissed {
                                count: CountWithChange::start_from(self.requested_deadline_missed_count,1)
                            } 
              );
            } 
          }
        } // for
        changes
      } // Some
    } // match
  } // fn

  pub fn handle_timed_event(&mut self) {
    while let Some(e) =  self.timed_event_timer.poll() {
      match e {
        TimedEvent::DeadlineMissedCheck => {
          self.handle_requested_deadline_event();
          self.set_requested_deadline_check_timer(); // re-prime timer
        }
      }
    }
  }

  pub fn process_command(&mut self) {
    trace!("process_command {:?}", self.my_guid );
    loop {
      use std::sync::mpsc::TryRecvError;
      match self.data_reader_command_receiver.try_recv() {
        Ok(ReaderCommand::RESET_REQUESTED_DEADLINE_STATUS) => {
          warn!("RESET_REQUESTED_DEADLINE_STATUS not implemented!");
          //TODO: This should be implemented.
        }

        // Disconnected is normal when terminating
        Err(TryRecvError::Disconnected) => { 
          trace!("DataReader disconnected");
          break
        }
        Err(TryRecvError::Empty) => {
          warn!("There was no command. Spurious command event??");
          break
        }
      }
    }
  }

  fn handle_requested_deadline_event(&mut self) {
    debug!("handle_requested_deadline_event");
    for missed_deadline in self.calculate_if_requested_deadline_is_missed() {
      self.send_status_change(missed_deadline);
    }
  }

  // Used for test/debugging purposes
  #[cfg(test)]
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
  #[cfg(test)]
  pub fn get_history_cache_sequence_start_and_end_numbers(&self) -> Vec<SequenceNumber> {
    let start = self.seqnum_instant_map.iter().min().unwrap().0;
    let end = self.seqnum_instant_map.iter().max().unwrap().0;
    return vec![*start, *end];
  }

 
  // updates or adds a new writer proxy, doesn't touch changes
  pub fn update_writer_proxy(&mut self, proxy: RtpsWriterProxy, offered_qos: QosPolicies) {
    debug!("update_writer_proxy topic={:?}",self.topic_name);
    match offered_qos.compliance_failure_wrt( &self.qos_policy ) {
      None => { // success, update or insert
        let writer_id = proxy.remote_writer_guid;
        let count_change =
          self.matched_writer_update(proxy); 
        if count_change > 0 {
          self.writer_match_count_total += count_change;
          self.send_status_change(DataReaderStatus::SubscriptionMatched{
              total: CountWithChange::new(self.writer_match_count_total, count_change ), 
              current: CountWithChange::new(self.matched_writers.len() as i32, count_change ),
          });
          info!("Matched new remote writer on topic={:?} writer= {:?}", 
                self.topic_name, writer_id);
        }
      }
      Some(bad_policy_id) => { // no QoS match
        self.offered_incompatible_qos_count += 1;
        self.send_status_change(DataReaderStatus::RequestedIncompatibleQos {
          count: CountWithChange::new(self.offered_incompatible_qos_count, 1),
          last_policy_id: bad_policy_id,
          policies: Vec::new(), // TODO. implementation missing
        });
        debug!("update_writer_proxy - QoS mismatch {:?}", bad_policy_id);
      }
    }
  }

  // return value counts how many new proxies were added
  fn matched_writer_update(&mut self, proxy: RtpsWriterProxy) -> i32 {
    match self.matched_writer_lookup(proxy.remote_writer_guid) {
      Some(op) => {
        op.update_contents(proxy); 
        0
      }
      None => {
        self.matched_writers.insert(proxy.remote_writer_guid, proxy); 
        1 
      }
    }
  }

  pub fn remove_writer_proxy(&mut self, writer_guid:GUID) {
    if self.matched_writers.contains_key(&writer_guid) {
      self.matched_writers.remove(&writer_guid);
      self.send_status_change(DataReaderStatus::SubscriptionMatched { 
                total: CountWithChange::new(self.writer_match_count_total , 0 ),
                current: CountWithChange::new(self.matched_writers.len() as i32 , -1)
              });
    }
  }

  // Entire remote participant was lost.
  // Remove all remote readers belonging to it.
  pub fn participant_lost(&mut self, guid_prefix: GuidPrefix) {
    let lost_readers : Vec<GUID> = 
      self.matched_writers.range( guid_prefix.range() )
        .map(|(g,_)| *g)
        .collect();
    for reader in lost_readers {
      self.remove_writer_proxy(reader)
    }
  }

  pub fn contains_writer(&self, entity_id: EntityId) -> bool {
    self
      .matched_writers
      .iter()
      .any(|(&g, _)| g.entityId == entity_id)
  }

  #[cfg(test)]
  pub(crate) fn matched_writer_add(
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
    self.update_writer_proxy(proxy, QosPolicies::qos_none() );
  }

  fn matched_writer_lookup(&mut self, remote_writer_guid: GUID) -> Option<&mut RtpsWriterProxy> {
    self.matched_writers.get_mut(&remote_writer_guid)
  }


  // handles regular data message and updates history cache
  pub fn handle_data_msg(&mut self, data: Data, 
      data_flags:BitFlags<DATA_Flags>, mr_state: MessageReceiverState ) 
  {
    //trace!("handle_data_msg entry");
    let receive_timestamp = Timestamp::now();

    let duration = match mr_state.timestamp {
      Some(ts) => receive_timestamp.duration_since(ts),
      None => Duration::DURATION_ZERO,
    };

    // checking lifespan for silent dropping of message
    if let Some(ls) = self.get_qos().lifespan {
      if ls.duration < duration {
        return;
      }
    }

    let writer_guid = GUID::new_with_prefix_and_id(mr_state.source_guid_prefix, data.writer_id);
    let writer_seq_num = data.writer_sn; // for borrow checker

    match Self::data_to_ddsdata(data,data_flags) {
      Ok(ddsdata) => {
        self.process_received_data(ddsdata, receive_timestamp, mr_state.timestamp, 
          writer_guid, writer_seq_num)    
      }
      Err(e) => debug!("Parsing DATA to DDSData failed: {}",e),
    }

    
  }

  pub fn handle_datafrag_msg(&mut self, datafrag: DataFrag, datafrag_flags: BitFlags<DATAFRAG_Flags>,
     mr_state: MessageReceiverState) 
  {
    let writer_guid = GUID::new_with_prefix_and_id(mr_state.source_guid_prefix, datafrag.writer_id);
    let seq_num = datafrag.writer_sn;
    let receive_timestamp = Timestamp::now();

    // check if this submessage is expired already
    if let (Some(source_timestamp), Some(lifespan)) = (mr_state.timestamp, self.get_qos().lifespan) {
      let elapsed = receive_timestamp.duration_since(source_timestamp);
      if lifespan.duration < elapsed {
        info!("DataFrag {:?} from {:?} lifespan exeeded. duration={:?} elapsed={:?}",
            seq_num, writer_guid, lifespan.duration, elapsed);
        return
      }
    }
    let writer_seq_num = datafrag.writer_sn; // for borrow checker
    if let Some(writer_proxy) = self.matched_writer_lookup(writer_guid) {
      if let Some(complete_ddsdata) = writer_proxy.handle_datafrag(datafrag, datafrag_flags) {
        // Source timestamp (if any) will be the timestamp of the last fragment (that completes the sample).
        self.process_received_data(complete_ddsdata, receive_timestamp, mr_state.timestamp, writer_guid, writer_seq_num );  
      } else {
        // not yet complete, nothing more to do
      }
    } else {
      info!("Reader got DATAFRAG, but I have no writer proxy")
    }
  }

  // common parts of processing DATA or a completed DATAFRAG (when all frags are received)
  fn process_received_data(&mut self, ddsdata:DDSData, receive_timestamp: Timestamp,
      source_timestamp: Option<Timestamp>, 
      writer_guid:GUID, writer_sn: SequenceNumber) 
  {
    trace!("handle_data_msg from {:?} seq={:?} topic={:?} stateful={:?}", 
        &writer_guid, writer_sn, self.topic_name, self.is_stateful,);
    if self.is_stateful {
      let my_entityid = self.my_guid.entityId; // to please borrow checker
      if let Some(writer_proxy) = self.matched_writer_lookup(writer_guid) {
        if writer_proxy.contains_change(writer_sn) {
          // change already present
          debug!("handle_data_msg already have this seq={:?}", writer_sn);
          if my_entityid == EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER {
            debug!("Accepting duplicate message to participant reader.");
            // This is an attmpted workaround to eProsima FastRTPS not
            // incrementing sequence numbers. (eProsime shapes demo 2.1.0 from 2021)
          } else {
            return 
          }
        }
        // Add the change and get the instant
        writer_proxy.received_changes_add(writer_sn, receive_timestamp);
      } else {
        // no writer proxy found
        info!("handle_data_msg in stateful Reader {:?} has no writer proxy for {:?} topic={:?}",
          my_entityid, writer_guid, self.topic_name, );
      }
    } else {
      // stateless reader
      todo!()
    }

    self.make_cache_change(ddsdata, receive_timestamp, source_timestamp, writer_guid, writer_sn);

    // Add to own track-keeping datastructure
    #[cfg(test)]
    self.seqnum_instant_map.insert(writer_sn, receive_timestamp);

    self.notify_cache_change();
  }

  fn data_to_ddsdata(data:Data, data_flags:BitFlags<DATA_Flags>) -> Result<DDSData,String> {
    let representation_identifier = 
      if data_flags.contains(DATA_Flags::Endianness) { RepresentationIdentifier::CDR_LE } 
      else { RepresentationIdentifier::CDR_BE };

    match (data.serialized_payload , 
        data_flags.contains(DATA_Flags::Data) , data_flags.contains(DATA_Flags::Key)) {
      (Some(sp), true, false ) => { // data
        Ok(DDSData::new(sp))
      }

      (Some(sp), false, true  ) => { // key
        Ok(DDSData::new_disposed_by_key(
            Self::deduce_change_kind(data.inline_qos, false, representation_identifier), 
            sp))
      }

      (None, false, false ) => { // no data, no key. Maybe there is inline QoS?
        // At least we should find key hash, or we do not know WTF the writer is talking about
        let key_hash = 
          match data.inline_qos
              .as_ref()
              .map( |iqos| InlineQos::key_hash(iqos).ok() )
              .flatten().flatten() {
            Some(h) => Ok(h),
            None => {
              info!("Received DATA that has no payload and no key_hash inline QoS - discarding");
              // Note: This case is normal when handling coherent sets.
              // The coherent set end marker is sent as DATA with no payload and not key, only Inline QoS.
              Err("DATA with no contents".to_string())
            }
          }?;
        // now, let's try to determine what is the dispose reason
        let change_kind = 
          Self::deduce_change_kind(data.inline_qos, false, representation_identifier);
        Ok(DDSData::new_disposed_by_key_hash(change_kind, key_hash ))
      }

      (Some(_), true , true  ) => { // payload cannot be both key and data.
        // RTPS Spec 9.4.5.3.1 Flags in the Submessage Header says
        // "D=1 and K=1 is an invalid combination in this version of the protocol."
        warn!("Got DATA that claims to be both data and key - discarding.");
        Err("Ambiguous data/key received.".to_string())
      }

      (Some(_), false, false ) => { // data but no data? - this should not be possible
        warn!("make_cache_change - Flags says no data or key, but got payload!");
        Err("DATA message has mystery contents".to_string())
      }
      (None, true, _ ) | (None, _ , true ) => {
        warn!("make_cache_change - Where is my SerializedPayload?");
        Err("DATA message contents missing".to_string())
      }
    }
  }


  // Returns if responding with ACKNACK?
  // TODO: Return value seems to go unused in callers.
  pub fn handle_heartbeat_msg(
    &mut self,
    heartbeat: Heartbeat,
    final_flag_set: bool,
    mr_state: MessageReceiverState,
  ) -> bool {
    let writer_guid =
      GUID::new_with_prefix_and_id(mr_state.source_guid_prefix, heartbeat.writer_id);

    if ! self.is_stateful {
      debug!("HEARTBEAT from {:?}, reader is stateless. Ignoring. topic={:?} reader={:?}", 
        writer_guid,self.topic_name, self.my_guid);
        return false
    }

    if !self.matched_writers.contains_key(&writer_guid) {
      info!("HEARTBEAT from {:?}, but no writer proxy available. topic={:?} reader={:?}", 
        writer_guid, self.topic_name, self.my_guid);
      return false
    }
    // sanity check
    if heartbeat.first_sn < SequenceNumber::default() {
      warn!("Writer {:?} advertised SequenceNumbers from {:?} to {:?}!",
          writer_guid, heartbeat.first_sn, heartbeat.last_sn);
    }

    let writer_proxy = match self.matched_writer_lookup(writer_guid) {
      Some(wp) => wp,
      None => {
        error!("Writer proxy disappeared 1!");
        return false
      } // Matching writer not found
    };

    let mut mr_state = mr_state;
    mr_state.unicast_reply_locator_list = writer_proxy.unicast_locator_list.clone();

    if heartbeat.count <= writer_proxy.received_heartbeat_count {
      // This heartbeat was already seen an processed.
      return false
    }
    writer_proxy.received_heartbeat_count = heartbeat.count;

    // remove fragmented changes until first_sn.
    let removed_instances = writer_proxy.irrelevant_changes_up_to(heartbeat.first_sn);

    // Remove instances from DDSHistoryCache
    {
      // Create local scope so that dds_cache write lock is dropped ASAP
      let mut cache = match self.dds_cache.write() {
        Ok(rwlock) => rwlock,
        // TODO: Should we panic here? Are we allowed to continue with poisoned DDSCache?
        Err(e) => panic!("The DDSCache of is poisoned. Error: {}", e),
      };
      for instant in removed_instances.values() {
        match cache.from_topic_remove_change(&self.topic_name, instant) {
          Some(_) => (),
          None => debug!("WriterProxy told to remove an instant which was not present"), // This may be normal?
        }
      }
    }

    // this is duplicate code from above, but needed, because we need another mutable borrow.
    // TODO: Maybe could be written in some sensible way.
    let writer_proxy = match self.matched_writer_lookup(writer_guid) {
      Some(wp) => wp,
      None => {
        error!("Writer proxy disappeared 2!");
        return false
      } // Matching writer not found
    };

    // See if ACKNACK is needed, and generate one.
    let missing_seqnums =
        writer_proxy.get_missing_sequence_numbers(heartbeat.first_sn, heartbeat.last_sn);
    
    // Interpretation of final flag in RTPS spec 
    // 8.4.2.3.1 Readers must respond eventually after receiving a HEARTBEAT with final flag not set
    // 
    // Upon receiving a HEARTBEAT Message with final flag not set, the Reader must respond 
    // with an ACKNACK Message. The ACKNACK Message may acknowledge having received all 
    // the data samples or may indicate that some data samples are missing.
    // The response may be delayed to avoid message storms.

    if ! missing_seqnums.is_empty() || ! final_flag_set {
      // report of what we have.
      // We claim to have received all SNs before "base" and produce a set of missing 
      // sequence numbers that are >= base.
      let reader_sn_state =
        match missing_seqnums.get(0)  {
          Some(&first_missing) => {
              // Here we assume missing_seqnums are returned in order.
              // Limit the set to maximum that can be sent in acknack submessage..
            SequenceNumberSet
              ::from_base_and_set(first_missing, 
                &BTreeSet::from_iter(missing_seqnums.iter().copied()
                                      .take_while( |sn| sn < &(first_missing + SequenceNumber::from(256)) ))
                )
            }

          // Nothing missing. Report that we have all we have.
          None => SequenceNumberSet::new_empty(writer_proxy.all_ackable_before()),           
        };


      let response_ack_nack = AckNack {
        reader_id: self.get_entity_id(),
        writer_id: heartbeat.writer_id,
        reader_sn_state,
        count: self.sent_ack_nack_count,
      };

      self.sent_ack_nack_count += 1;

      // Sanity check
      if response_ack_nack.reader_sn_state.base() > heartbeat.last_sn + SequenceNumber::new(1) {
        error!("OOPS! AckNack sanity check tripped: HEARTBEAT = {:?} ACKNACK = {:?}",
          &heartbeat, &response_ack_nack
          );
      }


      // The acknack can be sent now or later. The rest of the RTPS message
      // needs to be constructed. p. 48
      self.send_acknack(response_ack_nack, mr_state);
      return true
    }
    
    false
  } // fn 

  pub fn handle_gap_msg(&mut self, gap: Gap, mr_state: MessageReceiverState) {
    // ATM all things related to groups is ignored. TODO?

    let writer_guid = GUID::new_with_prefix_and_id(mr_state.source_guid_prefix, gap.writer_id);

    if ! self.is_stateful {
      debug!("GAP from {:?}, reader is stateless. Ignoring. topic={:?} reader={:?}", 
        writer_guid,self.topic_name, self.my_guid);
        return
    }

    let writer_proxy = match self.matched_writer_lookup(writer_guid) {
      Some(wp) => wp,
      None => {
        info!("GAP from {:?}, but no writer proxy available. topic={:?} reader={:?}", 
        writer_guid, self.topic_name, self.my_guid);
        return
      } // Matching writer not found
    };

    // Sequencenumber set in the gap is invalid: (section 8.3.5.5)
    // Set checked to be not empty. Unwraps won't panic

    // TODO: Implement SequenceNumberSet rules validation (section 8.3.5.5)
    // Already in the SequenceNumber module.

    // TODO: Implement GAP rules validation (Section 8.3.7.4.3) here.

    // if gap.gap_list

    // if gap.gap_list.set.len() != 0 {
    //   if i64::from(gap.gap_start) < 1i64
    //     || (gap.gap_list.set.iter().max().unwrap() - gap.gap_list.set.iter().min().unwrap()) >= 256
    //   {
    //     return;
    //   }
    // } else {
    //   return;
    // };



    // Irrelevant sequence numbers communicated in the Gap message are
    // composed of two groups:
    //   1. All sequence numbers in the range gapStart <= sequence_number < gapList.base
    let mut removed_changes : BTreeSet<Timestamp> = 
      writer_proxy.irrelevant_changes_range(gap.gap_start, gap.gap_list.base())
        .values().copied()
        .collect();

    //   2. All the sequence numbers that appear explicitly listed in the gapList.
    for seq_num in gap.gap_list.iter() {
      writer_proxy.set_irrelevant_change(seq_num)
        .map( |t| removed_changes.insert(t) );
    }

    // Remove from DDSHistoryCache
    let mut cache = match self.dds_cache.write() {
      Ok(rwlock) => rwlock,
      // TODO: Should we panic here? Are we allowed to continue with poisoned DDSCache?
      Err(e) => panic!("The DDSCache of is poisoned. Error: {}", e),
    };
    for instant in &removed_changes {
      cache.from_topic_remove_change(&self.topic_name, instant);
    }

    // Is this needed?
    // self.notify_cache_change();
  }


  pub fn handle_heartbeatfrag_msg(
    &mut self,
    _heartbeatfrag: HeartbeatFrag,
    _mr_state: MessageReceiverState,
  ) {
    todo!()
  }

  // This is used to determine exact change kind in case we do not get a data payload in DATA submessage
  fn deduce_change_kind(inline_qos: Option<ParameterList>, no_writers:bool , ri:RepresentationIdentifier ) 
    -> ChangeKind
  {

    match inline_qos
        .as_ref().map( |iqos| InlineQos::status_info(iqos, ri).ok())
        .flatten() {
      Some(si) => 
        si.change_kind(), // get from inline QoS
        // TODO: What if si.change_kind() gives ALIVE ??
      None => { 
        if no_writers { ChangeKind::NotAliveUnregistered } 
        else { ChangeKind::NotAliveDisposed } // TODO: Is this reasonable default?
      }
    }
  }

  // Convert DATA submessage into a CacheChange and update history cache
  fn make_cache_change(
    &mut self,
    data: DDSData,
    receive_timestamp: Timestamp,
    source_timestamp: Option<Timestamp>,
    writer_guid: GUID,
    writer_sn: SequenceNumber,
  ) {

    let cache_change = CacheChange::new(writer_guid, writer_sn, source_timestamp, data);
    let mut cache = match self.dds_cache.write() {
      Ok(rwlock) => rwlock,
      // TODO: Should we panic here? Are we allowed to continue with poisoned DDSCache?
      Err(e) => panic!("The DDSCache of is poisoned. Error: {}", e),
    };
    cache.to_topic_add_change(&self.topic_name, &receive_timestamp, cache_change);
  }

  // notifies DataReaders (or any listeners that history cache has changed for this reader)
  // likely use of mio channel
  pub fn notify_cache_change(&self) {
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
    // Indicate our endianness.
    // Set final flag to indicate that we are NOT requesting immediate heartbeat response.
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
    self.udp_sender.send_to_locator_list(&bytes, &mr_state.unicast_reply_locator_list);
  }

  // TODO: This is much duplicate code from send_acknack.
  pub fn send_preemptive_acknacks(&mut self) {
    let flags = BitFlags::<ACKNACK_Flags>::from_flag(ACKNACK_Flags::Endianness);
    // Do not set final flag --> we are requesting immediate heartbeat from writers.

    let infodst_flags =
      BitFlags::<INFODESTINATION_Flags>::from_flag(INFODESTINATION_Flags::Endianness);

    self.sent_ack_nack_count += 1;

    for (_, writer_proxy) in self
      .matched_writers
      .iter()
      .filter(|(_, p)| p.no_changes() )
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
        reader_sn_state: SequenceNumberSet::new_empty(SequenceNumber::from(1)),
        count: self.sent_ack_nack_count,
      };

      match info_dst.create_submessage(infodst_flags) {
        Some(m) => message.add_submessage(m),
        None => continue, //TODO: is this correct??
      };

      match acknack.create_submessage(flags) {
        Some(m) => message.add_submessage(m),
        None => continue, //TODO: ??
      };

      let bytes = message
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap();
      self.udp_sender.send_to_locator_list(&bytes, &writer_proxy.unicast_locator_list);
    }
  }

  pub fn topic_name(&self) -> &String {
    &self.topic_name
  }
} // impl

impl HasQoSPolicy for Reader {
  fn get_qos(&self) -> QosPolicies {
    self.qos_policy.clone()
  }
}

impl RTPSEntity for Reader {
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
  use crate::structure::guid::{GUID, EntityId};
  use crate::messages::submessages::submessage_elements::serialized_payload::{SerializedPayload};
  use crate::structure::guid::{GuidPrefix, EntityKind};
  use crate::dds::statusevents::DataReaderStatus;
  use crate::structure::topic_kind::TopicKind;
  use crate::dds::typedesc::TypeDesc;

  #[test]
  fn rtpsreader_notification() {
    let mut guid = GUID::dummy_test_guid(EntityKind::READER_NO_KEY_USER_DEFINED);
    guid.entityId = EntityId::createCustomEntityID([1, 2, 3], EntityKind::from(111));

    let (send, rec) = mio_channel::sync_channel::<()>(100);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<DataReaderStatus>(100);
    let (_reader_command_sender, reader_command_receiver) =
      mio_channel::sync_channel::<ReaderCommand>(10);

    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));
    dds_cache.write().unwrap().add_new_topic(
      &"test".to_string(),
      TopicKind::NoKey,
      TypeDesc::new("testi"),
    );

    let reader_ing = ReaderIngredients {
      guid,
      notification_sender: send,
      status_sender: status_sender,
      topic_name: "test".to_string(),
      qos_policy: QosPolicies::qos_none(),
      data_reader_command_receiver: reader_command_receiver,
    };
    let mut reader = Reader::new(
      reader_ing,
      dds_cache,
      Rc::new(UDPSender::new(0).unwrap()),
    );

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(&[1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], EntityKind::WRITER_WITH_KEY_USER_DEFINED),
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
    data.reader_id = EntityId::createCustomEntityID([1, 2, 3], EntityKind::from(111));
    data.writer_id = writer_guid.entityId;

    reader.handle_data_msg(data, BitFlags::<DATA_Flags>::empty(), mr_state);

    assert!(rec.try_recv().is_ok());
  }

  #[test]
  fn rtpsreader_handle_data() {
    let new_guid = GUID::default();

    let (send, rec) = mio_channel::sync_channel::<()>(100);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<DataReaderStatus>(100);
    let (_reader_command_sender, reader_command_receiver) =
      mio_channel::sync_channel::<ReaderCommand>(10);

    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));
    dds_cache.write().unwrap().add_new_topic(
      &"test".to_string(),
      TopicKind::NoKey,
      TypeDesc::new("testi"),
    );

    let reader_ing = ReaderIngredients {
      guid: new_guid,
      notification_sender: send,
      status_sender: status_sender,
      topic_name: "test".to_string(),
      qos_policy: QosPolicies::qos_none(),
      data_reader_command_receiver: reader_command_receiver,
    };
    let mut new_reader = Reader::new(
      reader_ing,
      dds_cache.clone(),
      Rc::new(UDPSender::new(0).unwrap()),
    );

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(&[1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], EntityKind::WRITER_WITH_KEY_USER_DEFINED),
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
    new_reader.handle_data_msg(d.clone(), BitFlags::<DATA_Flags>::empty(), mr_state);

    assert!(rec.try_recv().is_ok());

    let hc_locked = dds_cache.read().unwrap();
    let cc_from_chache = hc_locked.from_topic_get_change(
      &new_reader.topic_name,
      &new_reader.seqnum_instant_map.get(&d_seqnum).unwrap(),
    );

    let ddsdata = DDSData::new(d.serialized_payload.unwrap());
    let cc_built_here = CacheChange::new( writer_guid, d_seqnum, None, ddsdata );

    assert_eq!(cc_from_chache.unwrap(), &cc_built_here);
  }

  #[test]
  fn rtpsreader_handle_heartbeat() {
    let new_guid = GUID::dummy_test_guid(EntityKind::READER_NO_KEY_USER_DEFINED);

    let (send, _rec) = mio_channel::sync_channel::<()>(100);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<DataReaderStatus>(100);
    let (_reader_command_sender, reader_command_receiver) =
      mio_channel::sync_channel::<ReaderCommand>(10);

    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));
    dds_cache.write().unwrap().add_new_topic(
      &"test".to_string(),
      TopicKind::NoKey,
      TypeDesc::new("testi"),
    );
    let reader_ing = ReaderIngredients {
      guid: new_guid,
      notification_sender: send,
      status_sender: status_sender,
      topic_name: "test".to_string(),
      qos_policy: QosPolicies::qos_none(),
      data_reader_command_receiver: reader_command_receiver,
    };
    let mut new_reader = Reader::new(
      reader_ing,
      dds_cache.clone(),
      Rc::new(UDPSender::new(0).unwrap()),
    );

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(&[1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], EntityKind::WRITER_WITH_KEY_USER_DEFINED),
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
      new_reader.get_guid(),
      SequenceNumber::from(1),
      None,
      d.clone(),
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
      new_reader.get_guid(),
      SequenceNumber::from(2),
      None,
      d.clone(),
    );
    new_reader.dds_cache.write().unwrap().to_topic_add_change(
      &new_reader.topic_name,
      &Timestamp::now(),
      change.clone(),
    );
    changes.push(change);

    let change = CacheChange::new(
      new_reader.get_guid(),
      SequenceNumber::from(3),
      None,
      d,
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
    let new_guid = GUID::dummy_test_guid(EntityKind::READER_NO_KEY_USER_DEFINED);
    let (send, _rec) = mio_channel::sync_channel::<()>(100);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<DataReaderStatus>(100);
    let (_reader_command_sender, reader_command_receiver) =
      mio_channel::sync_channel::<ReaderCommand>(10);

    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));
    dds_cache.write().unwrap().add_new_topic(
      &"test".to_string(),
      TopicKind::NoKey,
      TypeDesc::new("testi"),
    );

    let reader_ing = ReaderIngredients {
      guid: new_guid,
      notification_sender: send,
      status_sender: status_sender,
      topic_name: "test".to_string(),
      qos_policy: QosPolicies::qos_none(),
      data_reader_command_receiver: reader_command_receiver,
    };
    let mut reader = Reader::new(
      reader_ing,
      dds_cache.clone(),
      Rc::new(UDPSender::new(0).unwrap()),
    );


    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(&[1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], EntityKind::WRITER_WITH_KEY_USER_DEFINED),
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
      reader.handle_data_msg(d.clone(), BitFlags::<DATA_Flags>::empty(), mr_state.clone());
      changes.push(
        reader
          .get_history_cache_change(d.writer_sn)
          .unwrap()
          .clone(),
      );
    }

    // make sequence numbers 1-3 and 5 7 irrelevant
    let mut gap_list = SequenceNumberSet::new(SequenceNumber::from(4),7);
    gap_list.insert(SequenceNumber::from(5)); 
    gap_list.insert(SequenceNumber::from(7));

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
