use crate::messages::submessages::data::Data;
use chrono::Duration as chronoDuration;
//use crate::messages::submessages::info_destination::InfoDestination;
use crate::messages::submessages::info_timestamp::InfoTimestamp;
use crate::structure::time::Timestamp;
use byteorder::{ByteOrder, LittleEndian};
use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::{header::Header, vendor_id::VendorId, protocol_id::ProtocolId};
use crate::structure::guid::{GuidPrefix, EntityId, GUID};
use crate::structure::sequence_number::{SequenceNumber};
use std::hash::Hasher;
use crate::{
  submessages::{
    Heartbeat, SubmessageHeader, SubmessageKind, SubmessageFlag, InterpreterSubmessage,
    EntitySubmessage, AckNack, InfoDestination,
  },
  structure::cache_change::{CacheChange},
  serialization::{SubMessage, Message},
};

use speedy::{Writable, Endianness};
use time::Timespec;
use time::get_time;
use mio_extras::channel as mio_channel;
use mio::Token;

use crate::dds::ddsdata::DDSData;
use crate::{
  network::udp_sender::UDPSender,
  structure::{
    entity::{Entity, EntityAttributes},
    endpoint::{EndpointAttributes, Endpoint},
    locator::{Locator, LocatorKind}, dds_cache::DDSCache, 
  }, common::heartbeat_handler::HeartbeatHandler,
};
use super::{rtps_reader_proxy::RtpsReaderProxy};
use std::{net::SocketAddr, time::{Instant, Duration}, sync::{RwLock, Arc}, collections::{HashSet, HashMap, BTreeMap, hash_map::DefaultHasher}};

pub struct Writer {
  pub submessage_count: usize,
  source_version: ProtocolVersion,
  source_vendor_id: VendorId,
  pub source_guid_prefix: GuidPrefix,
  pub dest_guid_prefix: GuidPrefix,
  pub endianness: Endianness,
  heartbeat_message_counter: i32,
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
  pub heartbeat_perioid: Duration,
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
  entity_attributes: EntityAttributes,
  cache_change_receiver: mio_channel::Receiver<DDSData>,
  ///The RTPS ReaderProxy class represents the information an RTPS StatefulWriter maintains on each matched
  ///RTPS Reader
  readers: Vec<RtpsReaderProxy>,
  message: Option<Message>,
  udp_sender: UDPSender,
  // This writer can read/write to only one of this DDSCache topic caches identified with my_topic_name
  dds_cache : Arc<RwLock<DDSCache>>,
  /// Writer can only read/write to this topic DDSHistoryCache.
  my_topic_name: String,
  /// Maps this writers local sequence numbers to DDSHistodyCache instants.
  /// Useful when negative acknack is recieved.
  sequence_number_to_instant : BTreeMap<SequenceNumber, Instant>,
 //// Maps this writers local sequence numbers to DDSHistodyCache instants.
  /// Useful when datawriter dispose is recieved.
  key_to_instant: HashMap<u64, Instant>,
  /// Set of disposed samples.
  /// Useful when reader requires some sample with acknack. 
  disposed_sequence_numbers : HashSet<SequenceNumber>,
  //When dataWriter sends cacheChange message with cacheKind is NotAlive_Disposed
  //this is set true. If Datawriter after disposing sends new cahceChanges this falg is then 
  //turned true.
  //When writer is in disposed state it needs to send StatusInfo_t (PID_STATUS_INFO) with DisposedFlag 
  //TODO are messages send every heartbeat or just once ????
  pub writer_is_disposed : bool,
  ///Contains timer that needs to be set to timeout with duration of self.heartbeat_perioid
  ///Heartbeat handler sends notification when timer is up via miochannel to poll in Dp_eventWrapper
  heartbeat_handler : Option<HeartbeatHandler>,
}

impl Writer {
  pub fn new(
    guid: GUID,
    cache_change_receiver: mio_channel::Receiver<DDSData>,
    dds_cache: Arc<RwLock<DDSCache>>,
    topic_name: String,
  ) -> Writer {
    let entity_attributes = EntityAttributes::new(guid);
    Writer {
      submessage_count: 0,
      source_version: ProtocolVersion::PROTOCOLVERSION_2_3,
      source_vendor_id: VendorId::VENDOR_UNKNOWN,
      source_guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
      dest_guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
      endianness: Endianness::LittleEndian,
      heartbeat_message_counter: 0,
      push_mode: true,
      heartbeat_perioid: Duration::from_millis(100),
      nack_respose_delay: Duration::from_millis(200),
      nack_suppression_duration: Duration::from_nanos(0),
      last_change_sequence_number: SequenceNumber::from(0),
      first_change_sequence_number: SequenceNumber::from(0),
      data_max_size_serialized: 999999999,
      entity_attributes,
      //enpoint_attributes: EndpointAttributes::default(),
      cache_change_receiver,
      readers: vec![
        /*
        RtpsReaderProxy::new_for_unit_testing(1000),
        RtpsReaderProxy::new_for_unit_testing(1001),
        RtpsReaderProxy::new_for_unit_testing(1002),*/
      ],
      message: None,
      endpoint_attributes: EndpointAttributes::default(),
      udp_sender: UDPSender::new_with_random_port(),
      dds_cache,
      my_topic_name : topic_name,
      sequence_number_to_instant : BTreeMap::new(),
      key_to_instant : HashMap::new(),
      disposed_sequence_numbers : HashSet::new(),
      writer_is_disposed : false,
      heartbeat_handler : None,
    }
    //entity_attributes.guid.entityId.
  }

  /// To know when token represents a writer we should look entity attribute kind
  pub fn get_entity_token(&self) -> Token {
    let id = self.as_entity().as_usize();
    Token(id)
  }

  pub fn get_heartbeat_entity_token(&self) -> Token{
    let mut hasher = DefaultHasher::new();
    let id = self.as_entity().as_usize() as u64;
    hasher.write(&id.to_le_bytes());
    let hashedID : u64 = hasher.finish();
    Token(hashedID as usize)    
  }

  pub fn cache_change_receiver(&self) -> &mio_channel::Receiver<DDSData> {
    &self.cache_change_receiver
  }

  pub fn add_heartbeat_handler(&mut self, heartbeat_handler: HeartbeatHandler) {
    self.heartbeat_handler = Some(heartbeat_handler);
    self.heartbeat_handler.as_mut().unwrap().set_timeout(&chronoDuration::from_std(self.heartbeat_perioid).unwrap());
  }

  /// this should be called everytime heartbeat message with token is recieved.
  pub fn handle_heartbeat_tick(&mut self){
    println!("HANDLE HERTBEAT writer entityID: {:?}", self.as_entity());

    let mut RTPSMessage: Message = Message::new();

    RTPSMessage.set_header(self.create_message_header());
    
    // TODO Set some guidprefix if needed at all.
    // Not sure if DST submessage and TS submessage are needed when sending heartbeat.
    
    //RTPSMessage.add_submessage(self.get_DST_submessage(GuidPrefix::GUIDPREFIX_UNKNOWN));
    //RTPSMessage.add_submessage(self.get_TS_submessage());
    RTPSMessage.add_submessage(self.get_heartbeat_msg());

    //let buffer :[u8] = RTPSMessage.write_to_vec_with_ctx(self.endianness).unwrap();
    for reader in &self.readers{
      if reader.unicast_locator_list.len() > 0 {
        self.send_unicast_message_to_reader(&RTPSMessage,reader);
      }
      if reader.multicast_locator_list.len() > 0 {
        self.send_multicast_message_to_reader(&RTPSMessage,reader);
      }
      
    }

    self.set_heartbeat_timer();
  }

  /// after heartbeat is handled timer should be set running again.
  fn set_heartbeat_timer(&mut self){
    self.heartbeat_handler.as_mut().unwrap().set_timeout(&chronoDuration::from_std(self.heartbeat_perioid).unwrap())
  }

  pub fn insert_to_history_cache(&mut self, data: DDSData) {
    self.writer_is_disposed = false;
    self.increase_last_change_sequence_number();
    // If first write then set first change sequence number to 1
    if self.first_change_sequence_number == SequenceNumber::from(0) {
      self.first_change_sequence_number = SequenceNumber::from(1);
    }
    let data_key = { data.value_key_hash.clone() };
    let datavalue = Some(data);
    let new_cache_change = CacheChange::new(
      self.as_entity().guid,
      self.last_change_sequence_number,
      datavalue,
    );
    let insta = Instant::now();
    self.dds_cache.write().unwrap().to_topic_add_change(
      &self.my_topic_name,
      &insta,
      new_cache_change,
    );
    self
      .sequence_number_to_instant
      .insert(self.last_change_sequence_number, insta.clone());
    self.key_to_instant.insert(data_key, insta.clone());
    self.writer_set_unsent_changes();
  }
  
  /// This needs to be called when dataWriter does dispose.
  /// This does not remove anything from datacahce but changes the status of writer to disposed.
  pub fn handle_not_alive_disposed_cache_change(&mut self, data : DDSData){
    let instant = self.key_to_instant.get(&data.value_key_hash);
    self.writer_is_disposed = true;
    if instant.is_some(){
      self.dds_cache.write().unwrap().from_topic_set_change_to_not_alive_disposed(&self.my_topic_name, &instant.unwrap());
    }
    
  }

  /// Removes permanently cacheChanges from DDSCache.
  /// CacheChanges can be safely removed only if they are acked by all readers.
  pub fn remove_all_acked_changes(&mut self){
  let acked_by_all_reades = {
    let mut acked_by_all_reades : Vec<(&Instant,&SequenceNumber)>  = vec![];
    for (sq,i)  in self.sequence_number_to_instant.iter(){
      if self.change_with_sequence_number_is_acked_by_all(&sq){
        acked_by_all_reades.push((i,sq));
      }
    }
    acked_by_all_reades
  };
  {
    for (i,_sq) in acked_by_all_reades{
      self.dds_cache.write().unwrap().from_topic_remove_change(&self.my_topic_name, i);
      // TODO MAYBE USEFUL TO REMOVE SEQUENCE NUMBERST THAT ARE REMOVED
      //self.sequence_number_to_instant.remove(sq);
    }
  }
     
  }

  fn remove_from_history_cache(&mut self, instant : &Instant){
    let removed_change = self.dds_cache.write().unwrap().from_topic_remove_change(&self.my_topic_name, instant);
    println!("removed change from DDShistoryCache {:?}",removed_change);
    if removed_change.is_some(){
      self.disposed_sequence_numbers.insert(removed_change.unwrap().sequence_number);
    }
    else{
      todo!();
    }
  }

  fn remove_from_history_cache_with_sequence_number(&mut self, sequence_number : &SequenceNumber){
    let instant = self.sequence_number_to_instant.get(sequence_number);
    if instant.is_some(){
      let removed_change = self.dds_cache.write().unwrap().from_topic_remove_change(&self.my_topic_name, instant.unwrap());
      if removed_change.is_none(){
        todo!("Cache change with seqnum {:?} and instant {:?} cold not be revod from DDSCache", sequence_number, instant)
      }
    }
    else{
      todo!("sequence number: {:?} cannot be tranformed to instant ", sequence_number);
    }
    
  }


  fn increase_last_change_sequence_number(&mut self) {
    self.last_change_sequence_number = self.last_change_sequence_number + SequenceNumber::from(1);
  }

  fn increase_heartbeat_counter(&mut self) {
    self.heartbeat_message_counter = self.heartbeat_message_counter + 1;
  }

  pub fn can_send_some(&self) -> bool {
    for reader_proxy in &self.readers {
      if reader_proxy.can_send() {
        return true;
      }
    }
    return false;
  }

  pub fn sequence_is_sent_to_all_readers(&self, sequence_number: SequenceNumber) -> bool {
    for reader_proxy in &self.readers {
      if reader_proxy.unsent_changes().contains(&sequence_number) {
        return false;
      }
    }
    return true;
  }

  pub fn sequence_needs_to_be_send_to(
    &self,
    sequence_number: SequenceNumber,
  ) -> Vec<&RtpsReaderProxy> {
    let mut readers_remaining: Vec<&RtpsReaderProxy> = vec![];
    for reader_proxy in &self.readers {
      if reader_proxy.unsent_changes().contains(&sequence_number) {
        readers_remaining.push(&reader_proxy);
      }
    }
    return readers_remaining;
  }

  fn get_next_reader_next_unsend_message(&mut self) -> (Option<Message>, Option<GUID>) {
    for reader_proxy in &mut self.readers {
      if reader_proxy.can_send() {
        let sequenceNumber = reader_proxy.next_unsent_change();
        let instant = self.sequence_number_to_instant.get(sequenceNumber.unwrap());
        let cache = self.dds_cache.read().unwrap();
        let change = cache.from_topic_get_change(&self.my_topic_name, &instant.unwrap());
        let message: Message;
        let reader_entity_id = reader_proxy.remote_reader_guid.entityId.clone();
        let remote_reader_guid = reader_proxy.remote_reader_guid.clone();
        {
          message = self.write_user_msg(change.unwrap().clone(), reader_entity_id);
        }
        return (Some(message), Some(remote_reader_guid));
      }
    }
    return (None, None);
  }

  fn get_next_reader_next_requested_message(&mut self) -> (Option<Message>, Option<GUID>) {
    for reader_proxy in &mut self.readers {
      if reader_proxy.can_send() {
        let sequenceNumber = reader_proxy.next_requested_change();
        let instant = self.sequence_number_to_instant.get(sequenceNumber.unwrap());
        let cache = self.dds_cache.read().unwrap();
        let change = cache.from_topic_get_change(&self.my_topic_name, &instant.unwrap());
        let message: Message;
        let reader_entity_id = reader_proxy.remote_reader_guid.entityId.clone();
        let remote_reader_guid = reader_proxy.remote_reader_guid.clone();
        {
          message = self.write_user_msg(change.unwrap().clone(), reader_entity_id);
        }
        return (Some(message), Some(remote_reader_guid));
      }
    }
    return (None, None);
  }

  fn send_next_unsend_message(&mut self) {
    println!("send next unsend message");
    let mut uni_cast_adresses: Vec<SocketAddr> = vec![];
    let mut multi_cast_locators: Vec<Locator> = vec![];
    let mut buffer: Vec<u8> = vec![];
    let mut context = self.endianness;

    if self.endianness == Endianness::BigEndian {
      context = Endianness::BigEndian
    }

    let (message, Guid) = self.get_next_reader_next_unsend_message();
    if message.is_some() && Guid.is_some() {
      let reader = self.matched_reader_lookup(Guid.unwrap().guidPrefix, Guid.unwrap().entityId);
      let messageSequenceNumbers = message
        .as_ref()
        .unwrap()
        .get_data_sub_message_sequence_numbers();
      if reader.is_some() {
        buffer = message.unwrap().write_to_vec_with_ctx(context).unwrap();

        let readerProx = { reader.unwrap() };
        for loc in readerProx.unicast_locator_list.iter() {
          if loc.kind == LocatorKind::LOCATOR_KIND_UDPv4 {
            uni_cast_adresses.push(loc.clone().to_socket_address())
          }
        }
        for loc in readerProx.multicast_locator_list.iter() {
          if loc.kind == LocatorKind::LOCATOR_KIND_UDPv4 {
            multi_cast_locators.push(loc.clone())
          }
        }
      }
      self.udp_sender.send_to_all(&buffer, &uni_cast_adresses);
      for l in multi_cast_locators {
        if l.kind == LocatorKind::LOCATOR_KIND_UDPv4 {
          let a = l.to_socket_address();
          // TODO: handle unwrap
          self.udp_sender.send_ipv4_multicast(&buffer, a).unwrap();
        }
      }
      self.increase_heartbeat_counter_and_remove_unsend_sequence_numbers(
        messageSequenceNumbers,
        &Guid,
      )
    }
  }

  fn send_unicast_message_to_reader(&self, message : &Message, reader : &RtpsReaderProxy){
    let buffer = message.write_to_vec_with_ctx(self.endianness).unwrap();
    self.udp_sender.send_to_locator_list(&buffer, &reader.unicast_locator_list)
  }

  fn send_multicast_message_to_reader(&self, message : &Message, reader : &RtpsReaderProxy){
    let buffer = message.write_to_vec_with_ctx(self.endianness).unwrap();
    for multiaddress in &reader.multicast_locator_list{
      if multiaddress.kind == LocatorKind::LOCATOR_KIND_UDPv4 {
        self.udp_sender.send_ipv4_multicast(&buffer, multiaddress.to_socket_address());
      }
      else if multiaddress.kind == LocatorKind::LOCATOR_KIND_UDPv6 {
        todo!();
      }
    }
  }

  pub fn send_all_unsend_messages(&mut self) {
    println!("Writer try to send all unsend messages");
    loop {
      let (message, guid) = self.get_next_reader_next_unsend_message();
      if message.is_some() && guid.is_some() {
        self.send_next_unsend_message();
      } else {
        break;
      }
    }
    println!("Writer all unsent messages have been sent");
  }

  fn create_submessage_header_flags(&self, kind: &SubmessageKind) -> SubmessageFlag {
    let mut sub_flags: SubmessageFlag = SubmessageFlag {
      flags: 0b0000000_u8,
    };
    // The first flag, the EndiannessFlag is in this position for all submessage types (2^0)
    if self.endianness == Endianness::LittleEndian {
      sub_flags.set_flag(1);
    }
    if self.endianness == Endianness::BigEndian {
      sub_flags.clear_flag(1);
    } //in data submessage third flag is DataFlag (2^2 = 4)
      //if message contains serialized payload this should be set
    if kind == &SubmessageKind::DATA {
      sub_flags.set_flag(4)
    }
    return sub_flags;
  }

  fn create_message_header(&self) -> Header {
    let head: Header = Header {
      protocol_id: ProtocolId::default(),
      protocol_version: ProtocolVersion {
        major: self.source_version.major,
        minor: self.source_version.minor,
      },
      vendor_id: VendorId {
        vendorId: self.source_vendor_id.vendorId,
      },
      guid_prefix: self.source_guid_prefix,
    };
    return head;
  }

  fn create_submessage_header(
    &self,
    kind: SubmessageKind,
    submessageLength: u16,
  ) -> SubmessageHeader {
    let sub_flags: SubmessageFlag = self.create_submessage_header_flags(&kind);
    let header: SubmessageHeader = SubmessageHeader {
      submessage_id: kind,
      flags: sub_flags,
      submessage_length: submessageLength,
    };
    header
  }

  pub fn get_TS_submessage(&self) -> SubMessage {
    let currentTime: Timespec = get_time();
    let timestamp = InfoTimestamp {
      timestamp: Timestamp::from(currentTime),
    };
    let mes = &mut timestamp.write_to_vec_with_ctx(self.endianness).unwrap();
    let size = mes.len();

    let submessageHeader = self.create_submessage_header(SubmessageKind::INFO_TS, size as u16);
    let s: SubMessage = SubMessage {
      header: submessageHeader,
      submessage: None,
      intepreterSubmessage: Some(InterpreterSubmessage::InfoTimestamp(
        timestamp,
        self.create_submessage_header_flags(&SubmessageKind::INFO_TS),
      )),
    };
    return s;
  }

  pub fn get_DST_submessage(&self, guid_prefix : GuidPrefix) -> SubMessage{
    //InfoDST length is always 12 because message contains only GuidPrefix
    let submessageHeader = self.create_submessage_header(SubmessageKind::INFO_DST, 12u16);
    let s: SubMessage = SubMessage {
      header: submessageHeader,
      submessage: None,
      intepreterSubmessage: Some(InterpreterSubmessage::InfoDestination(
        InfoDestination{
          guid_prefix : guid_prefix 
        }
        
      )),
    };
    return s;
  }

  pub fn get_DATA_msg_from_cache_change(
    &self,
    change: CacheChange,
    reader_entity_id: EntityId,
  ) -> SubMessage {
    let mut data_message = Data::new();
    let mut representationIdentifierBytes: [u8; 2] = [0, 0];
    if self.endianness == Endianness::LittleEndian {
      representationIdentifierBytes = [0x00, 0x01];
    } else if self.endianness == Endianness::BigEndian {
      representationIdentifierBytes = [0x00, 0x00];
    }
    data_message.serialized_payload.representation_identifier =
      LittleEndian::read_u16(&representationIdentifierBytes);
    //The current version of the protocol (2.3) does not use the representation_options: The sender shall set the representation_options to zero.
    data_message.serialized_payload.representation_options = 0u16;
    data_message.serialized_payload.value = change.data_value.unwrap().value.clone();
    data_message.reader_id = reader_entity_id;
    data_message.writer_sn = change.sequence_number;

    let size = data_message
      .write_to_vec_with_ctx(self.endianness)
      .unwrap()
      .len() as u16;
    let s: SubMessage = SubMessage {
      header: self.create_submessage_header(SubmessageKind::DATA, size),
      submessage: Some(crate::submessages::EntitySubmessage::Data(
        data_message,
        self.create_submessage_header_flags(&SubmessageKind::DATA),
      )),
      intepreterSubmessage: None,
    };
    return s;
  }

  pub fn get_heartbeat_msg(&self) -> SubMessage {
    let first = self.first_change_sequence_number;
    let last = self.last_change_sequence_number;

    let heartbeat = Heartbeat {
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: self.entity_attributes.guid.entityId,
      first_sn: first,
      last_sn: last,
      count: self.heartbeat_message_counter,
    };

    let mes = &mut heartbeat.write_to_vec_with_ctx(self.endianness).unwrap();
    let size = mes.len();
    let head = self.create_submessage_header(SubmessageKind::HEARTBEAT, size as u16);

    let s: SubMessage = SubMessage {
      header: head,
      intepreterSubmessage: None,
      submessage: Some(EntitySubmessage::Heartbeat(
        heartbeat,
        self.create_submessage_header_flags(&SubmessageKind::HEARTBEAT),
      )),
    };
    return s;
  }

  pub fn write_user_msg(&self, change: CacheChange, reader_entity_id: EntityId) -> Message {
    let mut message: Vec<u8> = vec![];

    let mut RTPSMessage: Message = Message::new();
    RTPSMessage.set_header(self.create_message_header());
    RTPSMessage.add_submessage(self.get_TS_submessage());
    let data = self.get_DATA_msg_from_cache_change(change.clone(), reader_entity_id);
    RTPSMessage.add_submessage(data);
    RTPSMessage.add_submessage(self.get_heartbeat_msg());
    message.append(&mut RTPSMessage.write_to_vec_with_ctx(self.endianness).unwrap());

    return RTPSMessage;
  }

  /// AckNack Is negative if reader_sn_state contains some sequenceNumbers in reader_sn_state set
  fn test_if_ack_nack_contains_not_recieved_sequence_numbers(&self, ack_nack: &AckNack) -> bool {
    if !&ack_nack.reader_sn_state.set.is_empty() {
      return true;
    }
    return false;
  }

  ///When receiving an ACKNACK Message indicating a Reader is missing some data samples, the Writer must
  //respond by either sending the missing data samples, sending a GAP message when the sample is not relevant, or
  //sending a HEARTBEAT message when the sample is no longer available
  pub fn handle_ack_nack(&mut self, guid_prefix: GuidPrefix, an: AckNack) {
    {
      let reader_proxy = self.matched_reader_lookup(guid_prefix, an.reader_id);

      // if ack nac says reader has recieved data then change history cache chage status ???
      if reader_proxy.is_none() {
        print!("reader proxy is not known!");
        panic!();
      }
    }
    if self.test_if_ack_nack_contains_not_recieved_sequence_numbers(&an) == false {
      let reader_proxy = self.matched_reader_lookup(guid_prefix, an.reader_id);
      reader_proxy
        .unwrap()
        .acked_changes_set(an.reader_sn_state.base);
    } else {
      // if ack nac says reader has NOT recieved data then add data to requested changes
      let reader_proxy = self.matched_reader_lookup(guid_prefix, an.reader_id);
      reader_proxy
        .unwrap()
        .add_requested_changes(an.reader_sn_state.set)
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

  pub fn matched_reader_remove(&mut self, reader_proxy: RtpsReaderProxy) {
    let pos = &self.readers.iter().position(|x| {
      x.remote_group_entity_id == reader_proxy.remote_group_entity_id
        && x.remote_reader_guid == reader_proxy.remote_reader_guid
    });
    if pos.is_some() {
      &self.readers.remove(pos.unwrap());
    }
  }

  ///This operation finds the ReaderProxy with GUID_t a_reader_guid from the set
  /// get guid Prefix from RTPS message main header
  /// get reader guid from AckNack submessage readerEntityId
  pub fn matched_reader_lookup(
    &mut self,
    guid_prefix: GuidPrefix,
    reader_entity_id: EntityId,
  ) -> Option<&mut RtpsReaderProxy> {
    let search_guid: GUID = GUID {
      guidPrefix: guid_prefix,
      entityId: reader_entity_id,
    };
    let pos = &self
      .readers
      .iter()
      .position(|x| x.remote_reader_guid == search_guid);
    if pos.is_some() {
      return Some(&mut self.readers[pos.unwrap()]);
    }
    return None;
  }

  ///This operation takes a CacheChange a_change as a parameter and determines whether all the ReaderProxy
  ///have acknowledged the CacheChange. The operation will return true if all ReaderProxy have acknowledged the
  ///corresponding CacheChange and false otherwise.
  pub fn is_acked_by_all(&self, cache_change: &CacheChange) -> bool {
    for _proxy in &self.readers {
      if _proxy.sequence_is_acked(&cache_change.sequence_number) == false {
        return false;
      }
    }
    return true;
  }

  pub fn change_with_sequence_number_is_acked_by_all(&self, sequence_number : &SequenceNumber) -> bool {
    for _proxy in &self.readers {
      if _proxy.sequence_is_acked(sequence_number) == false {
        return false;
      }
    }
    return true;
  }

  pub fn increase_heartbeat_counter_and_remove_unsend_sequence_numbers(
    &mut self,
    sequence_numbers: Vec<SequenceNumber>,
    remote_reader_guid: &Option<GUID>,
  ) {
    let sequenceNumbersCount: usize = { sequence_numbers.len() };
    if remote_reader_guid.is_some() {
      let readerProxy = self.matched_reader_lookup(
        remote_reader_guid.unwrap().guidPrefix,
        remote_reader_guid.unwrap().entityId,
      );
      if readerProxy.is_some() {
        let reader_prox = readerProxy.unwrap();
        for SequenceNumber in sequence_numbers {
          reader_prox.remove_unsend_change(SequenceNumber);
        }
      }
    }
    if remote_reader_guid.is_some() {
      for _x in 0..sequenceNumbersCount {
        self.increase_heartbeat_counter();
      }
    }
  }

  pub fn increase_heartbeat_counter_and_remove_unsend(
    &mut self,
    message: &Option<Message>,
    remote_reader_guid: &Option<GUID>,
  ) {
    if message.is_some() {
      let sequence_numbers = message
        .as_ref()
        .unwrap()
        .get_data_sub_message_sequence_numbers();
      for sq in sequence_numbers {
        let readerProxy = self
          .matched_reader_lookup(
            remote_reader_guid.unwrap().guidPrefix,
            remote_reader_guid.unwrap().entityId,
          )
          .unwrap();
        readerProxy.remove_unsend_change(sq)
      }
    }
  }

  pub fn writer_set_unsent_changes(&mut self) {
    for reader in &mut self.readers {
      reader.unsend_changes_set(self.last_change_sequence_number.clone());
    }
  }

  /*
  // TODO Used for test/debugging purposes
  pub fn get_history_cache_change_data(&self, sequence_number: SequenceNumber) -> Option<Vec<u8>> {
    println!(
      "history cache !!!! {:?}",
      self.history_cache.get_change(sequence_number).unwrap()
    );
    let a = (*self
      .history_cache
      .get_change(sequence_number)
      .unwrap()
      .data_value
      .clone()
      .unwrap()
      .value)
      .to_vec()
      .clone();
    return Some(a);
  }
  // Used for test/debugging purposes
  pub fn get_history_cache_change(&self, sequence_number: SequenceNumber) -> &CacheChange {
    println!(
      "history cache !!!! {:?}",
      self.history_cache.get_change(sequence_number).unwrap()
    );
    let a = self.history_cache.get_change(sequence_number).unwrap();
    return a;
  }

  // TODO Used for test/debugging purposes
  pub fn get_history_cache_sequence_start_and_end_numbers(&self) -> Vec<SequenceNumber> {
    let start = self.history_cache.get_seq_num_min();
    let end = self.history_cache.get_seq_num_max();
    return vec![start.unwrap().clone(), end.unwrap().clone()];
  }



  // TODO Used for test/debugging purposes CAN BE DELETED WHEN tests are created again.
  pub fn handle_new_dds_data_message(&mut self, sample: DDSData) {
    self.increase_last_change_sequence_number();
    let dds_data = sample;
    let change = CacheChange::new(
      self.get_guid().clone(),
      self.last_change_sequence_number,
      Some(dds_data),
    );
    self.history_cache.add_change(change);
    for reader in &mut self.readers {
      reader.unsend_changes_set(self.last_change_sequence_number.clone());
    }
  }
  */
}

impl Entity for Writer {
  fn as_entity(&self) -> &crate::structure::entity::EntityAttributes {
    &self.entity_attributes
  }
}

impl Endpoint for Writer {
  fn as_endpoint(&self) -> &crate::structure::endpoint::EndpointAttributes {
    &self.endpoint_attributes
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    dds::{qos::QosPolicies, participant::DomainParticipant, typedesc::TypeDesc},
  };
  use std::thread;
  use crate::test::random_data::*;

  /*
  #[test]

  fn create_writer_add_readers_create_messages() {
    let new_guid = GUID::new();
    let (_sender, reciever) = mio_channel::channel::<DDSData>();
    let mut writer = Writer::new(new_guid, reciever, Arc::new(RwLock::new(DDSCache::new())), String::from("topicName123ABC"));
    let instance_handle = writer.history_cache.generate_free_instance_handle();
    let instance_handle2 = writer.history_cache.generate_free_instance_handle();
    let dds_data: DDSData = DDSData::new(instance_handle, SerializedPayload::default());
    let dds_data2: DDSData = DDSData::new(instance_handle2, SerializedPayload::default());

    let reader_proxy1: RtpsReaderProxy = RtpsReaderProxy::new(GUID::new());
    let reader_proxy2: RtpsReaderProxy = RtpsReaderProxy::new(GUID::new());
    let reader_proxy3: RtpsReaderProxy = RtpsReaderProxy::new(GUID::new());

    // add readers
    writer.matched_reader_add(reader_proxy1);
    writer.matched_reader_add(reader_proxy2);
    writer.matched_reader_add(reader_proxy3);

    assert_eq!(writer.can_send_some(), false);
    // make change to history cahce
    writer.handle_new_dds_data_message(dds_data);
    assert_eq!(writer.can_send_some(), true);

    // Three messages can be generated
    let RTPS1 = writer.get_next_reader_next_unsend_message();
    writer.increase_heartbeat_counter_and_remove_unsend(&RTPS1.0, &RTPS1.1);

    let RTPS2 = writer.get_next_reader_next_unsend_message();
    writer.increase_heartbeat_counter_and_remove_unsend(&RTPS2.0, &RTPS2.1);

    let RTPS3 = writer.get_next_reader_next_unsend_message();
    writer.increase_heartbeat_counter_and_remove_unsend(&RTPS3.0, &RTPS3.1);

    let isSendToAll = writer.sequence_is_sent_to_all_readers(SequenceNumber::from(1));
    let reaminingReaders = writer.sequence_needs_to_be_send_to(SequenceNumber::from(1));

    println!("is Send To All  {:?}", isSendToAll);
    println!("need to be send to {:?}", reaminingReaders);

    assert_eq!(writer.can_send_some(), false);

    writer.handle_new_dds_data_message(dds_data2);
    assert_eq!(writer.can_send_some(), true);

    // Three messages can be ganerated
    let RTPS4 = writer.get_next_reader_next_unsend_message();
    writer.increase_heartbeat_counter_and_remove_unsend(&RTPS4.0, &RTPS4.1);

    let RTPS5 = writer.get_next_reader_next_unsend_message();
    writer.increase_heartbeat_counter_and_remove_unsend(&RTPS5.0, &RTPS5.1);

    let RTPS6 = writer.get_next_reader_next_unsend_message();
    writer.increase_heartbeat_counter_and_remove_unsend(&RTPS6.0, &RTPS6.1);

    let reaminingReaders2 = writer.sequence_needs_to_be_send_to(SequenceNumber::from(2));

    println!("need to be send to {:?}", reaminingReaders2);
    assert_eq!(writer.can_send_some(), false);
  }
  */

  #[test]
  fn test_writer_recieves_datawriter_cache_change_notifications() {
    let domain_participant = DomainParticipant::new(4, 0);
    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());

    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos)
      .expect("Failed to create topic");
    let mut data_writer = publisher
      .create_datawriter(None, &topic, &qos)
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
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    let writeResult = data_writer.write(data, None).expect("Unable to write data");

    println!("writerResult:  {:?}", writeResult);
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    let writeResult = data_writer
      .write(data2, None)
      .expect("Unable to write data");

    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    println!("writerResult:  {:?}", writeResult);
    let writeResult = data_writer
      .write(data3, None)
      .expect("Unable to write data");

    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    println!("writerResult:  {:?}", writeResult);
  }
}
