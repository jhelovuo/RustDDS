use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::vendor_id::VendorId;
use crate::messages::submessages::submessage_header::SubmessageHeader;
use crate::messages::submessages::submessage_kind::SubmessageKind;
use crate::messages::submessages::submessages::EntitySubmessage;
use crate::structure::guid::{GuidPrefix, GUID};
use crate::structure::entity::Entity;
use crate::structure::locator::{LocatorKind, LocatorList, Locator};
use crate::structure::time::Time;
use crate::serialization::submessage::SubMessage;
use crate::messages::submessages::data::Data;


use crate::messages::submessages::info_destination::InfoDestination;
use crate::messages::submessages::info_source::InfoSource;
use crate::messages::submessages::info_reply::InfoReply;
use crate::messages::submessages::info_timestamp::InfoTimestamp;

use crate::dds::reader::Reader;
use crate::structure::guid::EntityId;
use crate::structure::sequence_number::{SequenceNumber, SequenceNumberSet};

use speedy::{Readable, Endianness};


const RTPS_MESSAGE_HEADER_SIZE: usize = 20;

#[derive(Debug, PartialEq)]
pub struct MessageReceiver {
  pub available_readers: Vec<Reader>,

  participant_guid_prefix: GuidPrefix,
  //submessage_vec: Vec<SubMessage>, //messages should be sent immideately, 
  //because subsequent interpreter submessages might change some receiver parameters

  pub source_version: ProtocolVersion,
  pub source_vendor_id: VendorId,
  pub source_guid_prefix: GuidPrefix,
  pub dest_guid_prefix: GuidPrefix,
  pub unicast_reply_locator_list: LocatorList,
  pub multicast_reply_locator_list: LocatorList,
  pub have_timestamp: bool,
  pub timestamp: Time,

  pos: usize,  
  pub submessage_count: usize,
}

impl MessageReceiver{
  pub fn new(participant_guid_prefix: GuidPrefix) -> MessageReceiver {
    // could be passed in as a parameter
    let locator_kind = LocatorKind::LOCATOR_KIND_UDPv4;

    MessageReceiver {
      available_readers: Vec::new(),
      participant_guid_prefix,

      //submessage_vec: Vec::new(),

      source_version: ProtocolVersion::PROTOCOLVERSION,
      source_vendor_id: VendorId::VENDOR_UNKNOWN,
      source_guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
      dest_guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
      unicast_reply_locator_list: vec![Locator {
        kind: locator_kind,
        address: Locator::LOCATOR_ADDRESS_INVALID,
        port: Locator::LOCATOR_PORT_INVALID,
      }],
      multicast_reply_locator_list: vec![Locator {
        kind: locator_kind,
        address: Locator::LOCATOR_ADDRESS_INVALID,
        port: Locator::LOCATOR_PORT_INVALID,
      }],
      have_timestamp: false,
      timestamp: Time::TIME_INVALID,
      
      pos: 0,
      submessage_count: 0,
    }
  }

  pub fn reset(&mut self){
      //self.submessage_vec.clear();

      self.source_version = ProtocolVersion::PROTOCOLVERSION;
      self.source_vendor_id = VendorId::VENDOR_UNKNOWN;
      self.source_guid_prefix = GuidPrefix::GUIDPREFIX_UNKNOWN;
      self.dest_guid_prefix = GuidPrefix::GUIDPREFIX_UNKNOWN;
      //self.unicast_reply_locator_list.clear();
      //self.multicast_reply_locator_list.clear();
      self.have_timestamp = false;
      self.timestamp = Time::TIME_INVALID;

      self.pos = 0;
      self.submessage_count = 0;
  }

  pub fn add_reader(& mut self, new_reader: Reader){ // or guidPRefix?
    match self.available_readers.iter().find(
      |&r| r.get_guid() == new_reader.get_guid()
    ) {
        None => {self.available_readers.push(new_reader);},
        Some(_) => {},
    }
  }

  pub fn remove_reader(&mut self, old_reader_guid: GUID){
    if let Some(pos) = self.available_readers.iter().position(
      |r| r.get_guid() == old_reader_guid
    ) {
        self.available_readers.remove(pos);
    }
  }

  fn get_reader(&mut self, reader_id: EntityId) -> Option<&mut Reader>{
    self.available_readers.iter_mut().find(
      |r| r.get_entity_id() == reader_id
    )
  }


  // TODO use for test and debugging only
  fn get_reader_and_history_cache_change<'a>(self, reader_id: EntityId, sequence_number : SequenceNumber) -> Option<Data>{
    //println!("readers: {:?}", self.available_readers);
    let reader = self.available_readers.iter().find(
      |r| r.get_entity_id() == reader_id
            
    ).unwrap();
    let a : Option<&Data> = reader.get_history_cache_change_data(sequence_number);
    return Some(a.unwrap().clone());
  }

  fn get_reader_history_cache_start_and_end_seq_num(&self, reader_id: EntityId) -> Vec<SequenceNumber>{
    let reader = self.available_readers.iter().find(
      |r| r.get_entity_id() == reader_id
            
    ).unwrap();
    let a  = reader.get_history_cache_sequence_start_and_end_numbers();
    return a;
    
  }

  pub fn handle_discovery_msg(&mut self, _msg: Vec<u8>) {
    // 9.6.2.2
    // The discovery message is just a data message. No need for the 
    // messageReceiver to handle it any differently here?

    // The SPDPbuiltinPArticipantReader receives the SPDPdiscoveredParticipantData
    // from the remove Participants
    unimplemented!();
  }

  pub fn handle_user_msg(&mut self, msg: Vec<u8>) {
    if msg.len() < RTPS_MESSAGE_HEADER_SIZE {
      return;
    }
    self.reset();
    self.dest_guid_prefix = self.participant_guid_prefix;
    
    if !self.handle_RTPS_header(&msg) {
      return; // Header not in correct form
    }
    let endian = Endianness::LittleEndian; // Should be read from message??
    
    // Go through each submessage
    while self.pos < msg.len() {

      let submessage_header = match SubMessage::deserialize_header(
      endian,
      &msg[self.pos..self.pos+4]) {
        Some(T) => T,
        None => {return;},// rule 1. Could not create submessage header
      };
      self.pos += 4; // Submessage header lenght is 4 bytes

      let mut submessage_length = submessage_header.submessage_length as usize;
      //println!("submessage length: {:?}", submessage_length);
      if submessage_length == 0 { 
        submessage_length = msg.len() - self.pos; // RTPS 8.3.3.2.3
      } else if submessage_length > msg.len() - self.pos { 
        return; // rule 2
      }

      if !self.is_interpreter_submessage(
        &submessage_header,
        endian,
        &msg[self.pos..(self.pos + submessage_length)],
      ){
        let entity_submessage = match SubMessage::deserialize_msg(
          &submessage_header,
          endian,
          &msg[self.pos..(self.pos + submessage_length)],
        ){
            Some(T) => T,
            None => {continue;}// rule 3 
          }; 

        let new_submessage = SubMessage {
          header: submessage_header,
          submessage: entity_submessage,
        };
        //self.submessage_vec.push(new_submessage);
        self.send_submessage(new_submessage.submessage);
        self.submessage_count += 1;
      }
      self.pos += submessage_length;
    } // end while
  }

  fn send_submessage(&mut self, submessage: EntitySubmessage) {
    println!("send submessage");
    if self.dest_guid_prefix != self.participant_guid_prefix{
      println!("Ofcourse, the example messages are not for this participant?");
      println!("dest_guid_prefix: {:?}", self.dest_guid_prefix);
      println!("participant guid: {:?}", self.participant_guid_prefix);
      return; // Wrong target received
    }
    println!("{:?}",submessage);
    // TODO! If reader_id == ENTITYID_UNKNOWN, message should be sent to all matched readers
    match submessage {
      EntitySubmessage::Data(data, _) => {
        println!("datamessage target reader: {:?}", data.reader_id);
        let target_reader = self.get_reader(data.reader_id).unwrap(); 
        target_reader.handle_data_msg(data);
      },
      EntitySubmessage::Heartbeat(heartbeat, flags) => {
        let target_reader = self.get_reader(heartbeat.reader_id).unwrap();
        let _ack_nack_sent = target_reader.handle_heartbeat_msg(
          heartbeat, 
          flags.is_flag_set(1), // final flag!?
        );
      },
      EntitySubmessage::Gap(gap) => {
        let target_reader = self.get_reader(gap.reader_id).unwrap();
        target_reader.handle_gap_msg(gap);
      },
      EntitySubmessage::AckNack(_, _) => {},
      EntitySubmessage::DataFrag(_, _) => {},
      EntitySubmessage::HeartbeatFrag(_) => {},
      EntitySubmessage::NackFrag(_) => {},
    }

  }

  fn is_interpreter_submessage(
    &mut self, 
    msgheader: &SubmessageHeader,
    context: Endianness,
    buffer: &[u8],
  ) -> bool {
    match msgheader.submessage_id {
      SubmessageKind::INFO_TS => {
        let info_ts: InfoTimestamp = InfoTimestamp::read_from_buffer_with_ctx(
          context, buffer,
        ).unwrap();
        if !msgheader.flags.is_flag_set(1){
          self.have_timestamp = true;
          self.timestamp = info_ts.timestamp;
        }
      },
      SubmessageKind::INFO_SRC => {
        let info_src: InfoSource = InfoSource::read_from_buffer_with_ctx(
          context, buffer,
        ).unwrap();
        self.source_guid_prefix = info_src.guid_prefix;
        self.source_version = info_src.protocol_version;
        self.source_vendor_id = info_src.vendor_id;
        self.unicast_reply_locator_list.clear(); // Or invalid?
        self.multicast_reply_locator_list.clear(); // Or invalid?
        self.have_timestamp = false;
      },
      SubmessageKind::INFO_REPLY | SubmessageKind::INFO_REPLY_IP4 => {
        let info_reply: InfoReply = InfoReply::read_from_buffer_with_ctx(
          context, buffer,
        ).unwrap();
        self.unicast_reply_locator_list = info_reply.unicast_locator_list;
        if msgheader.flags.is_flag_set(1) {
          self.multicast_reply_locator_list = info_reply.multicast_locator_list
          .unwrap();
        } else {
          self.multicast_reply_locator_list.clear();
        }
      },
      SubmessageKind::INFO_DST => {
        let info_dest: InfoDestination = InfoDestination::read_from_buffer_with_ctx(
          context, buffer
        ).unwrap();
        if info_dest.guid_prefix != GUID::GUID_UNKNOWN.guidPrefix {
          self.dest_guid_prefix = info_dest.guid_prefix;
        } else {
          self.dest_guid_prefix = self.participant_guid_prefix;
        }
      },
      _ => return false,
    }
    self.submessage_count += 1;
    true
}

  fn handle_RTPS_header(
    &mut self, 
    msg: &[u8]
  ) -> bool {
    // First 4 bytes 'R' 'T' 'P' 'S'
    if msg[0] != 0x52 || msg[1] != 0x54 || msg[2] != 0x50 || msg[3] != 0x53 {
      return false;
    } 
    self.pos += 4;

    self.source_version = ProtocolVersion::read_from_buffer(
      &msg[self.pos..self.pos+2]).unwrap();
    self.pos += 2;

    // Check and set protocl version
    if self.source_version.major > ProtocolVersion::PROTOCOLVERSION.major {
      // Error: Too new version
      return false;
    }
    // Set vendor id
    self.source_vendor_id = VendorId::read_from_buffer(
      &msg[self.pos..self.pos+2]).unwrap();
    self.pos += 2;

    // Set source guid prefix
    self.source_guid_prefix = GuidPrefix::read_from_buffer(
      &msg[self.pos..self.pos+12]).unwrap();
    self.pos += 12;
    true
  }

} // impl messageReceiver

#[cfg(test)]

mod tests{
  use super::*;
  use crate::messages::header::Header;
  use crate::speedy::{Writable, Readable};
  use mio::{Ready, Registration, Poll, PollOpt, Token, SetReadiness};
  use crate::structure::sequence_number::{SequenceNumber, SequenceNumberSet};
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
  use serde::{Serialize, Deserialize};

  #[test]

  fn test_shapes_demo_message_deserialization(){

    // Data message should contain Shapetype values.
    // caprured with wireshark from shapes demo.
    // Udp packet with INFO_DST, INFO_TS, DATA, HEARTBEAT
    let udp_bits1: Vec<u8> = vec! [
        0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78,
        0x34, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03,
        0x00, 0x0c, 0x29, 0x2d, 0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x09, 0x01, 0x08,
        0x00, 0x1a, 0x15, 0xf3, 0x5e, 0x00, 0xcc, 0xfb, 0x13, 0x15, 0x05, 0x2c, 0x00,
        0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x01, 0x02, 0x00,
        0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x69, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00,
        0x00, 0x1e, 0x00, 0x00, 0x00, 0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07,
        0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00
      ];
      
    // this guid prefix is set here because exaple message target is this.
    let guiPrefix = GuidPrefix::new(vec![0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 0x31, 0xa2, 0x28, 0x20, 0x02, 0x8]);

    let mut message_receiver = 
      MessageReceiver::new(guiPrefix);

      let entity = EntityId::createCustomEntityID([0,0,0],7);
      let new_guid = GUID::new_with_prefix_and_id(guiPrefix,entity);
      new_guid.from_prefix(entity);
      let new_reader = Reader::new(new_guid);
  

    message_receiver.add_reader(new_reader);

    message_receiver.handle_user_msg(udp_bits1);  

    assert_eq!(message_receiver.submessage_count, 4);

    // this is not correct way to read history cache values but it serves as a test
    let sequenceNumbers = message_receiver.get_reader_history_cache_start_and_end_seq_num(new_guid.entityId);
    println!("history change sequence number range: {:?}",sequenceNumbers );

    let mut a = message_receiver.get_reader_and_history_cache_change(new_guid.entityId, *sequenceNumbers.first().unwrap()).unwrap();
    println!("reader history chache DATA: {:?}" , a.serialized_payload.value);

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct ShapeType<'a> {
      color: &'a str,
      x: i32,
      y: i32,
      size: i32,
    }

    let deserializedShapeType : ShapeType = deserialize_from_little_endian(&mut a.serialized_payload.value).unwrap();
    println!("deserialized shapeType: {:?}",deserializedShapeType);
    assert_eq!(deserializedShapeType.color, "RED");

  }



  #[test]
  fn mr_test_submsg_count(){
    // Udp packet with INFO_DST, INFO_TS, DATA, HEARTBEAT
    let udp_bits1: Vec<u8> = vec![
       0x52, 0x54, 0x50, 0x53, 0x02, 0x03,  
       0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 
       0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01,   
       0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 
       0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x09, 0x01,   
       0x08, 0x00, 0x18, 0x15, 0xf3, 0x5e, 0x00, 0x5c, 
       0xf0, 0x34, 0x15, 0x05, 0x2c, 0x00, 0x00, 0x00,   
       0x10, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 
       0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x43, 0x00,   
       0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x00, 
       0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x21, 0x00,   
       0x00, 0x00, 0x89, 0x00, 0x00, 0x00, 0x1e, 0x00, 
       0x00, 0x00, 0x07, 0x01, 0x1c, 0x00, 0x00, 0x00,  
       0x00, 0x07, 0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 
       0x00, 0x00, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00,   
       0x00, 0x00, 0x43, 0x00, 0x00, 0x00, 0x07, 0x00, 
       0x00, 0x00                     
    ];
    // Udp packet with INFO_DST, ACKNACK
    let udp_bits2: Vec<u8> = vec![
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03,
      0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 
      0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01,
      0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x06, 0x03,
      0x18, 0x00, 0x00, 0x00, 0x04, 0xc7, 0x00, 0x00, 
      0x04, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 
      0x00, 0x00
    ];

    let guid_new = GUID::new();
    let mut message_receiver = 
      MessageReceiver::new(guid_new.guidPrefix);

    message_receiver.handle_user_msg(udp_bits1);
    assert_eq!(message_receiver.submessage_count, 4);

    message_receiver.handle_user_msg(udp_bits2);
    assert_eq!(message_receiver.submessage_count, 2);
  }

  #[test]
  fn mr_test_header() {

    let guid_new = GUID::new();
    let header = Header::new(guid_new.guidPrefix);
    
    let bytes = header.write_to_vec().unwrap();
    let new_header = Header::read_from_buffer(&bytes).unwrap();
    assert_eq!(header, new_header);
  }

  #[test]
  fn mr_test_is_RTPS_header() {
    let test_header: Vec<u8> = vec![
      0x52, 0x54, 0x50, 0x53, 
      0x02, 0x03, 0x01, 0x0f,
      0x01, 0x0f, 0x99, 0x06, 
      0x78, 0x34, 0x00, 0x00, 
      0x01, 0x00, 0x00, 0x00, 
    ];
    let guid_new = GUID::new();
    let mut message_receiver = 
    MessageReceiver::new(guid_new.guidPrefix);
    assert!(message_receiver.handle_RTPS_header(&test_header));
  }
}
