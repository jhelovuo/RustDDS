use crate::structure::history_cache::HistoryCache;
use crate::messages::submessages::data::Data;
//use crate::messages::submessages::info_destination::InfoDestination;
use crate::messages::submessages::info_timestamp::InfoTimestamp;
use crate::structure::time::Timestamp;
use byteorder::{ByteOrder, LittleEndian};
use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::{header::Header, vendor_id::VendorId, protocol_id::ProtocolId};
use crate::structure::guid::{GuidPrefix, EntityId, GUID};
use crate::structure::sequence_number::{SequenceNumber};
use crate::{
  submessages::{
    Heartbeat, SubmessageHeader, SubmessageKind, SubmessageFlag, InterpreterSubmessage,
    EntitySubmessage, AckNack,
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
use crate::structure::{ entity::{Entity, EntityAttributes}};
use super::rtps_reader_proxy::RtpsReaderProxy;

pub struct RecievingReaderContact{

}

pub struct Writer {
  pub history_cache: HistoryCache,
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
  ///Optional attribute that indicates
  ///the maximum size of any
  ///SerializedPayload that may be
  ///sent by the Writer
  pub data_max_size_serialized : u64,

  //pub unicast_locator_list: LocatorList,
  //pub multicast_locator_list: LocatorList,

  entity_attributes: EntityAttributes,
  cache_change_receiver: mio_channel::Receiver<DDSData>,
  
  readers : Vec<RtpsReaderProxy>,

  message : Option<Message>,
}

impl Writer {
  pub fn new(guid: GUID, cache_change_receiver: mio_channel::Receiver<DDSData>) -> Writer {
    let entity_attributes = EntityAttributes::new(guid);
    Writer {
      history_cache: HistoryCache::new(),
      submessage_count : 0,
      source_version : ProtocolVersion::PROTOCOLVERSION_2_3,
      source_vendor_id : VendorId::VENDOR_UNKNOWN,
      source_guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
      dest_guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
      //participant_guid_prefix :participant_guid_prefix,
      //writer_id : writer_id,
      endianness : Endianness::LittleEndian,
      heartbeat_message_counter : 0,
      
      push_mode : true,
      heartbeat_perioid : Duration::from_secs(1),
      nack_respose_delay: Duration::from_millis(200),
      nack_suppression_duration: Duration::from_nanos(0),
      last_change_sequence_number :SequenceNumber::from(0),
      data_max_size_serialized : 999999999,
      entity_attributes,
      //enpoint_attributes: EndpointAttributes::default(),
      cache_change_receiver,
      readers : vec![],
      message : None,
    }
  }

  pub fn get_entity_token(&self) -> Token {
    let id = self.as_entity().as_usize();
    Token(id)
  }

  pub fn cache_change_receiver(&self) -> &mio_channel::Receiver<DDSData> {
    &self.cache_change_receiver
  }

  pub fn insert_to_history_cache(&mut self, data: DDSData) {
    let seq_num_max = self
      .history_cache
      .get_seq_num_max()
      .unwrap_or(&SequenceNumber::from(0))
      .clone();

    let datavalue = Some(data);

    let new_cache_change = CacheChange::new(self.as_entity().guid, seq_num_max, datavalue);
    self.history_cache.add_change(new_cache_change);
  }

  fn increase_last_change_sequence_number(&mut self){
    self.last_change_sequence_number = self.last_change_sequence_number + SequenceNumber::from(1);
  }

  pub fn can_send_some(&self) -> bool {
    for reader_proxy in &self.readers{
      if reader_proxy.can_send() {return true;}
    }
    return false;
  }

  pub fn sequence_is_sent_to_all_readers(&self, sequence_number : SequenceNumber) -> bool{
    for reader_proxy in &self.readers{
      if reader_proxy.unsent_changes().contains(&sequence_number){
        return false;
      }
    }
    return true; 
  }

  pub fn sequence_needs_to_be_send_to(&self, sequence_number : SequenceNumber) -> Vec<&RtpsReaderProxy>{
    let mut readers_remaining : Vec<&RtpsReaderProxy> = vec![];
    for reader_proxy in &self.readers{
      if reader_proxy.unsent_changes().contains(&sequence_number){
        readers_remaining.push(&reader_proxy);
      }
    }
    return readers_remaining; 
  }

  pub fn get_next_reader_next_unsend_message (&mut self) -> (Option<Message>, Option<GUID> ){
    for reader_proxy in &mut self.readers{
      if reader_proxy.can_send() 
      {
        let sequenceNumber = reader_proxy.next_unsent_change();
        let change = self.history_cache.get_change(*sequenceNumber.unwrap());
        let message:Message;
        let reader_entity_id = reader_proxy.remote_reader_guid.entityId.clone();
        let remote_reader_guid = reader_proxy.remote_reader_guid.clone();
        {
          message = self.write_user_msg(change.unwrap().clone(),reader_entity_id);
        }
        return ( Some(message),Some(remote_reader_guid));
      }
    }
    return (None, None);
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

  pub fn get_DATA_msg_from_cache_change(&self, change: CacheChange, reader_entity_id : EntityId) -> SubMessage {
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
    data_message.serialized_payload.value = (*change.data_value.unwrap().data()).clone();
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

  pub fn get_heartbeat_msg(& self) -> SubMessage {
    let mut first = SequenceNumber::from(1);
    let mut last = SequenceNumber::from(0);
    if self.history_cache.get_seq_num_min().is_some() {
      first = *self.history_cache.get_seq_num_min().unwrap();
    }
    if self.history_cache.get_seq_num_max().is_some() {
      last = *self.history_cache.get_seq_num_max().unwrap();
    }
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

  pub fn write_user_msg(& self, change: CacheChange, reader_entity_id : EntityId) -> Message {
    let mut message: Vec<u8> = vec![];

    let mut RTPSMessage: Message = Message::new();
    RTPSMessage.set_header(self.create_message_header());

    RTPSMessage.add_submessage(self.get_TS_submessage());
    let data = self.get_DATA_msg_from_cache_change(change.clone(), reader_entity_id);
    RTPSMessage.add_submessage(data);
    RTPSMessage.add_submessage(self.get_heartbeat_msg());

    println!("RTPS message: {:?}", RTPSMessage);
    message.append(&mut RTPSMessage.write_to_vec_with_ctx(self.endianness).unwrap());

    println!("serialized RTPS message: {:?}", message);
    return RTPSMessage;
  }


  /// AckNack Is negative if reader_sn_state contains some sequenceNumbers in reader_sn_state set
  fn test_if_ack_nack_contains_not_recieved_sequence_numbers(&self, ack_nack: &AckNack) -> bool{
    if ! &ack_nack.reader_sn_state.set.is_empty(){
      return true;
    }
    return false;
  }

  ///When receiving an ACKNACK Message indicating a Reader is missing some data samples, the Writer must
  //respond by either sending the missing data samples, sending a GAP message when the sample is not relevant, or
  //sending a HEARTBEAT message when the sample is no longer available
  pub fn handle_ack_nack(& mut self, guid_prefix : GuidPrefix, an: AckNack ){
    {
      let reader_proxy  = self.matched_reader_lookup(guid_prefix, an.reader_id);

      // if ack nac says reader has recieved data then change history cache chage status ???
      if reader_proxy.is_none(){
        print!("reader proxy is not known!");
        panic!();
   }
    }
    if self.test_if_ack_nack_contains_not_recieved_sequence_numbers(&an) == false{
      todo!()
      //an.reader_sn_state.base
    }
    else{
      // if ack nac says reader has NOT recieved data then add data to requested changes
      let reader_proxy  = self.matched_reader_lookup(guid_prefix, an.reader_id);
      reader_proxy.unwrap().add_requested_changes(an.reader_sn_state.set)
    }
  }

  pub fn matched_reader_add(&mut self, reader_proxy : RtpsReaderProxy ){
    if self.readers.iter().any(|x| x.remote_group_entity_id == reader_proxy.remote_group_entity_id && x.remote_reader_guid == reader_proxy.remote_reader_guid ){
      panic!("Reader proxy with same group entityid and remotereader guid added already");
    };
    &self.readers.push(reader_proxy);
  }

  pub fn matched_reader_remove(&mut self, reader_proxy : RtpsReaderProxy){
    let pos = &self.readers.iter().position(|x|x.remote_group_entity_id == reader_proxy.remote_group_entity_id && x.remote_reader_guid == reader_proxy.remote_reader_guid);
    if pos.is_some(){
      &self.readers.remove(pos.unwrap());
    }
  }

  ///This operation finds the ReaderProxy with GUID_t a_reader_guid from the set
  /// get guid Prefix from RTPS message main header
  /// get reader guid from AckNack submessage readerEntityId
  pub fn matched_reader_lookup(&mut self, guid_prefix : GuidPrefix , reader_entity_id : EntityId) -> Option<&mut RtpsReaderProxy>{
    let search_guid : GUID = GUID { guidPrefix : guid_prefix, entityId : reader_entity_id};
    let pos = &self.readers.iter().position(|x|x.remote_reader_guid == search_guid);
    if pos.is_some(){
      println!("mathed lookup is Some");

      return Some(&mut self.readers[pos.unwrap()]);
      //return self.readers[pos];
    }
    return None;
  }

  ///This operation takes a CacheChange a_change as a parameter and determines whether all the ReaderProxy
  ///have acknowledged the CacheChange. The operation will return true if all ReaderProxy have acknowledged the
  ///corresponding CacheChange and false otherwise.
  pub fn is_acked_by_all(&self, _cache_change : &CacheChange) -> bool{
    for _proxy in &self.readers{
      
    }
    todo!();
  }

  pub fn increase_heartbeat_counter_and_remove_unsend(&mut self, message: Option<Message>, remote_reader_guid : Option<GUID> ){
    if message.is_some() {
      for mes in message.unwrap().submessages(){
        if mes.submessage.is_some() {
          let entity_sub_message = mes.submessage.unwrap();
          let maybeDataMessage = entity_sub_message.get_data_submessage();
          if maybeDataMessage.is_some(){
            let sequenceNumber = maybeDataMessage.unwrap().writer_sn;

            self.heartbeat_message_counter = self.heartbeat_message_counter + 1;
            
            let readerProxy = self.matched_reader_lookup(remote_reader_guid.unwrap().guidPrefix, remote_reader_guid.unwrap().entityId).unwrap();
            readerProxy.remove_unsend_change(sequenceNumber)
          }
        }
      }
    }
  }

  // TODO Used for test/debugging purposes
  pub fn get_history_cache_change_data(&self, sequence_number: SequenceNumber) -> Option<DDSData> {
    println!(
      "history cache !!!! {:?}",
      self.history_cache.get_change(sequence_number).unwrap()
    );
    let a = self
      .history_cache
      .get_change(sequence_number)
      .unwrap()
      .data_value
      .clone();
    return a;
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


  /// This should be called when mio channel from datawriter recieves a new message
  pub fn handle_new_dds_data_message(&mut self, sample :DataSample<DDSData>) {
    self.increase_last_change_sequence_number();
    let dds_data = sample.value.unwrap();
    let change = CacheChange::new(self.get_guid(),self.last_change_sequence_number, Some(dds_data));
    self.history_cache.add_change(change);
    for reader in &mut self.readers{
      reader.unsend_changes_set(self.last_change_sequence_number.clone());
    }
  }

}

impl Entity for Writer {
  fn as_entity(&self) -> &crate::structure::entity::EntityAttributes {
    &self.entity_attributes
  }
}

/*
impl Endpoint for Writer {
  fn as_endpoint(&self) -> &crate::structure::endpoint::EndpointAttributes {
    &self.enpoint_attributes
  }
}
*/

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{messages::submessages::submessage_elements::serialized_payload::SerializedPayload, dds::{traits::key::DefaultKey}};

  #[test]
  fn create_writer_add_readers_create_messages(){
    let new_guid = GUID::new();
    let (sender, reciever) = mio_channel::channel::<DataSample<DDSData>>();
    let mut writer = Writer::new(new_guid,reciever);
    let dds_data : Option<DDSData> = Some(DDSData::new(DefaultKey::new(123123),SerializedPayload::default()));
    let dds_data2 : Option<DDSData> = Some(DDSData::new(DefaultKey::new(123124343),SerializedPayload::default()));
    let message_from_data_writer : DataSample<DDSData> = DataSample::new(Timestamp::from(Timespec::new(1424545, 123)),dds_data);
    let message_from_data_writer2 : DataSample<DDSData> = DataSample::new(Timestamp::from(Timespec::new(142454512312, 123333)),dds_data2);
    
   
    let reader_proxy1 : RtpsReaderProxy = RtpsReaderProxy::new(GUID::new());
    let reader_proxy2 : RtpsReaderProxy = RtpsReaderProxy::new(GUID::new());
    let reader_proxy3 : RtpsReaderProxy = RtpsReaderProxy::new(GUID::new());
    
    // add readers
    writer.matched_reader_add(reader_proxy1);
    writer.matched_reader_add(reader_proxy2);
    writer.matched_reader_add(reader_proxy3);

   
    assert_eq!(writer.can_send_some(), false);
    // make change to history cahce
    writer.handle_new_dds_data_message(message_from_data_writer);
    assert_eq!(writer.can_send_some(), true);

    // Three messages can be ganerated
    let RTPS1 = writer.get_next_reader_next_unsend_message();
    writer.increase_heartbeat_counter_and_remove_unsend(RTPS1.0,RTPS1.1);

    let RTPS2 = writer.get_next_reader_next_unsend_message();
    writer.increase_heartbeat_counter_and_remove_unsend(RTPS2.0,RTPS2.1);

    let RTPS3 = writer.get_next_reader_next_unsend_message();
    writer.increase_heartbeat_counter_and_remove_unsend(RTPS3.0,RTPS3.1);

    let isSendToAll = writer.sequence_is_sent_to_all_readers(SequenceNumber::from(1));
    let reaminingReaders = writer.sequence_needs_to_be_send_to(SequenceNumber::from(1));

    println!("is Send To All  {:?}",isSendToAll);
    println!("need to be send to {:?}", reaminingReaders);

    assert_eq!(writer.can_send_some(), false);

    writer.handle_new_dds_data_message(message_from_data_writer2);
    assert_eq!(writer.can_send_some(), true);

     // Three messages can be ganerated
     let RTPS4 = writer.get_next_reader_next_unsend_message();
     writer.increase_heartbeat_counter_and_remove_unsend(RTPS4.0,RTPS4.1);
 
     let RTPS5 = writer.get_next_reader_next_unsend_message();
     writer.increase_heartbeat_counter_and_remove_unsend(RTPS5.0,RTPS5.1);
 
     let RTPS6 = writer.get_next_reader_next_unsend_message();
     writer.increase_heartbeat_counter_and_remove_unsend(RTPS6.0,RTPS6.1);

     let reaminingReaders2 = writer.sequence_needs_to_be_send_to(SequenceNumber::from(2));

     println!("need to be send to {:?}", reaminingReaders2);
     assert_eq!(writer.can_send_some(), false);
  }
}


