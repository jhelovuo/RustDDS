use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::vendor_id::VendorId;
use crate::messages::submessages::submessage_header::SubmessageHeader;
use crate::messages::submessages::submessage_kind::SubmessageKind;
use crate::messages::submessages::submessages::EntitySubmessage;
use crate::structure::guid::{GuidPrefix, GUID};
use crate::structure::entity::Entity;
use crate::structure::locator::{LocatorKind, LocatorList, Locator};
use crate::structure::time::{Time, Timestamp};
use crate::serialization::submessage::SubMessage;

use crate::messages::submessages::info_destination::InfoDestination;
use crate::messages::submessages::info_source::InfoSource;
use crate::messages::submessages::info_reply::InfoReply;
use crate::messages::submessages::info_timestamp::InfoTimestamp;

use crate::dds::reader::Reader;
use crate::dds::ddsdata::DDSData;
use crate::structure::guid::EntityId;
use crate::structure::{
  cache_change::CacheChange,
  sequence_number::{SequenceNumber},
};

use speedy::{Readable, Endianness};

const RTPS_MESSAGE_HEADER_SIZE: usize = 20;

#[derive(Debug, PartialEq)]
pub struct MessageReceiver {
  pub available_readers: Vec<Reader>,

  own_guid_prefix: GuidPrefix,
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

impl MessageReceiver {
  pub fn new(participant_guid_prefix: GuidPrefix) -> MessageReceiver {
    // could be passed in as a parameter
    let locator_kind = LocatorKind::LOCATOR_KIND_UDPv4;

    MessageReceiver {
      available_readers: Vec::new(),
      own_guid_prefix: participant_guid_prefix,

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

  pub fn reset(&mut self) {
    self.source_version = ProtocolVersion::PROTOCOLVERSION;
    self.source_vendor_id = VendorId::VENDOR_UNKNOWN;
    self.source_guid_prefix = GuidPrefix::GUIDPREFIX_UNKNOWN;
    self.dest_guid_prefix = GuidPrefix::GUIDPREFIX_UNKNOWN;
    self.unicast_reply_locator_list.clear();
    self.multicast_reply_locator_list.clear();
    self.have_timestamp = false;
    self.timestamp = Time::TIME_INVALID;

    self.pos = 0;
    self.submessage_count = 0;
  }

  fn give_message_receiver_info(&self) -> MessageReceiverState {
    MessageReceiverState {
      //own_guid_prefix: self.own_guid_prefix,
      source_guid_prefix: self.source_guid_prefix,
      unicast_reply_locator_list: self.unicast_reply_locator_list.clone(),
      multicast_reply_locator_list: self.multicast_reply_locator_list.clone(),
      have_timestamp: self.have_timestamp,
      timestamp: self.timestamp,
    }
  }

  pub fn add_reader(&mut self, new_reader: Reader) {
    match self
      .available_readers
      .iter()
      .find(|&r| r.get_guid() == new_reader.get_guid())
    {
      None => {
        self.available_readers.push(new_reader);
      }
      Some(_) => {}
    }
  }

  pub fn remove_reader(&mut self, old_reader_guid: GUID) {
    if let Some(pos) = self
      .available_readers
      .iter()
      .position(|r| *r.get_guid() == old_reader_guid)
    {
      self.available_readers.remove(pos);
    }
  }

  fn get_reader(&mut self, reader_id: EntityId) -> Option<&mut Reader> {
    self
      .available_readers
      .iter_mut()
      .find(|r| *r.get_entity_id() == reader_id)
  }

  // TODO use for test and debugging only
  fn get_reader_and_history_cache_change(
    &self,
    reader_id: EntityId,
    sequence_number: SequenceNumber,
  ) -> Option<DDSData> {
    //println!("readers: {:?}", self.available_readers);
    let reader = self
      .available_readers
      .iter()
      .find(|&r| *r.get_entity_id() == reader_id)
      .unwrap();
    let a: Option<DDSData> = reader.get_history_cache_change_data(sequence_number);
    Some(a.unwrap().clone())
  }

  // TODO use for test and debugging only
  fn get_reader_and_history_cache_change_object(
    &self,
    reader_id: EntityId,
    sequence_number: SequenceNumber,
  ) -> CacheChange {
    //println!("readers: {:?}", self.available_readers);
    let reader = self
      .available_readers
      .iter()
      .find(|&r| *r.get_entity_id() == reader_id)
      .unwrap();
    let a = reader.get_history_cache_change(sequence_number).unwrap();
    a.clone()
  }

  fn get_reader_history_cache_start_and_end_seq_num(
    &self,
    reader_id: EntityId,
  ) -> Vec<SequenceNumber> {
    let reader = self
      .available_readers
      .iter()
      .find(|&r| *r.get_entity_id() == reader_id)
      .unwrap();
    reader.get_history_cache_sequence_start_and_end_numbers()
  }

  pub fn handle_discovery_msg(&mut self, _msg: Vec<u8>) {
    // 9.6.2.2
    // The discovery message is just a data message. No need for the
    // messageReceiver to handle it any differently here?
    unimplemented!();
  }

  pub fn handle_user_msg(&mut self, msg: Vec<u8>) {
    println!("handle user Message");
    if msg.len() < RTPS_MESSAGE_HEADER_SIZE {
      return;
    }
    self.reset();
    self.dest_guid_prefix = self.own_guid_prefix;

    if !self.handle_RTPS_header(&msg) {
      println!("Header not in correct form");
      return; // Header not in correct form
    }
    let endian = Endianness::LittleEndian; // Should be read from message??
    println!("Header in correct form");

    // Go through each submessage
    while self.pos < msg.len() {
      let submessage_header =
        match SubMessage::deserialize_header(endian, &msg[self.pos..self.pos + 4]) {
          Some(T) => T,
          None => {
            print!("could not create submessage header");
            return;
          } // rule 1. Could not create submessage header
        };
      self.pos += 4; // Submessage header lenght is 4 bytes

      let mut submessage_length = submessage_header.submessage_length as usize;
      println!("submessage length: {:?}", submessage_length);
      println!("submessage header: {:?}", submessage_header);
      if submessage_length == 0 {
        submessage_length = msg.len() - self.pos; // RTPS 8.3.3.2.3
      } else if submessage_length > msg.len() - self.pos {
        println!("submessage is longer than msg len ?????");
        return; // rule 2
      }

      if !self.handle_interpreter_submessage(
        &submessage_header,
        endian,
        &msg[self.pos..(self.pos + submessage_length)],
      ) {
        let entity_submessage = match SubMessage::deserialize_msg(
          &submessage_header,
          endian,
          &msg[self.pos..(self.pos + submessage_length)],
        ) {
          Some(entity_sm) => entity_sm,
          None => {
            continue; // rule 3
          }
        };

        let new_submessage = SubMessage {
          header: submessage_header,
          submessage: Some(entity_submessage),
          intepreterSubmessage: None,
        };

        self.send_submessage(new_submessage.submessage.unwrap());
        self.submessage_count += 1;
      }
      self.pos += submessage_length;
    } // end while
  }

  fn send_submessage(&mut self, submessage: EntitySubmessage) {
    if self.dest_guid_prefix != self.own_guid_prefix {
      println!("Ofcourse, the example messages are not for this participant?");
      println!("dest_guid_prefix: {:?}", self.dest_guid_prefix);
      println!("participant guid: {:?}", self.own_guid_prefix);
      return; // Wrong target received
    }

    // TODO! If reader_id == ENTITYID_UNKNOWN, message should be sent to all matched readers
    let mr_state = self.give_message_receiver_info();
    match submessage {
      EntitySubmessage::Data(data, _) => {
        let target_reader = self.get_reader(data.reader_id).unwrap();
        target_reader.handle_data_msg(data, mr_state);
      }
      EntitySubmessage::Heartbeat(heartbeat, flags) => {
        let target_reader = self.get_reader(heartbeat.reader_id).unwrap();
        target_reader.handle_heartbeat_msg(
          heartbeat,
          flags.is_flag_set(1), // final flag!?
          mr_state,
        );
      }
      EntitySubmessage::Gap(gap) => {
        let target_reader = self.get_reader(gap.reader_id).unwrap();
        target_reader.handle_gap_msg(gap, mr_state);
      }
      EntitySubmessage::AckNack(_, _) => {}
      EntitySubmessage::DataFrag(datafrag, _) => {
        let target_reader = self.get_reader(datafrag.reader_id).unwrap();
        target_reader.handle_datafrag_msg(datafrag, mr_state);
      }
      EntitySubmessage::HeartbeatFrag(_) => {}
      EntitySubmessage::NackFrag(_) => {}
    }
  }

  fn handle_interpreter_submessage(
    &mut self,
    msgheader: &SubmessageHeader,
    context: Endianness,
    buffer: &[u8],
  ) -> bool {
    match msgheader.submessage_id {
      SubmessageKind::INFO_TS => {
        let info_ts: InfoTimestamp =
          InfoTimestamp::read_from_buffer_with_ctx(context, buffer).unwrap();
        if !msgheader.flags.is_flag_set(1) {
          self.have_timestamp = true;
          self.timestamp = info_ts.timestamp;
        }
      }
      SubmessageKind::INFO_SRC => {
        let info_src: InfoSource = InfoSource::read_from_buffer_with_ctx(context, buffer).unwrap();
        self.source_guid_prefix = info_src.guid_prefix;
        self.source_version = info_src.protocol_version;
        self.source_vendor_id = info_src.vendor_id;
        self.unicast_reply_locator_list.clear(); // Or invalid?
        self.multicast_reply_locator_list.clear(); // Or invalid?
        self.have_timestamp = false;
      }
      SubmessageKind::INFO_REPLY | SubmessageKind::INFO_REPLY_IP4 => {
        let info_reply: InfoReply = InfoReply::read_from_buffer_with_ctx(context, buffer).unwrap();
        self.unicast_reply_locator_list = info_reply.unicast_locator_list;
        if msgheader.flags.is_flag_set(1) {
          self.multicast_reply_locator_list = info_reply.multicast_locator_list.unwrap();
        } else {
          self.multicast_reply_locator_list.clear();
        }
      }
      SubmessageKind::INFO_DST => {
        let info_dest: InfoDestination =
          InfoDestination::read_from_buffer_with_ctx(context, buffer).unwrap();
        if info_dest.guid_prefix != GUID::GUID_UNKNOWN.guidPrefix {
          self.dest_guid_prefix = info_dest.guid_prefix;
        } else {
          self.dest_guid_prefix = self.own_guid_prefix;
        }
      }
      _ => return false,
    }
    self.submessage_count += 1;
    true
  }

  fn handle_RTPS_header(&mut self, msg: &[u8]) -> bool {
    // First 4 bytes 'R' 'T' 'P' 'S'
    if msg[0] != 0x52 || msg[1] != 0x54 || msg[2] != 0x50 || msg[3] != 0x53 {
      return false;
    }
    self.pos += 4;

    self.source_version = ProtocolVersion::read_from_buffer(&msg[self.pos..self.pos + 2]).unwrap();
    self.pos += 2;

    // Check and set protocl version
    if self.source_version.major > ProtocolVersion::PROTOCOLVERSION.major {
      // Error: Too new version
      return false;
    }
    // Set vendor id
    self.source_vendor_id = VendorId::read_from_buffer(&msg[self.pos..self.pos + 2]).unwrap();
    self.pos += 2;

    // Set source guid prefix
    self.source_guid_prefix = GuidPrefix::read_from_buffer(&msg[self.pos..self.pos + 12]).unwrap();
    self.pos += 12;
    true
  }
} // impl messageReceiver
#[derive(Debug, Clone)]
pub struct MessageReceiverState {
  //pub own_guid_prefix: GuidPrefix,
  pub source_guid_prefix: GuidPrefix,
  pub unicast_reply_locator_list: LocatorList,
  pub multicast_reply_locator_list: LocatorList,
  pub have_timestamp: bool,
  pub timestamp: Time,
}

impl Default for MessageReceiverState {
  fn default() -> Self {
    Self {
      //own_guid_prefix: GuidPrefix::default(),
      source_guid_prefix: GuidPrefix::default(),
      unicast_reply_locator_list: LocatorList::default(),
      multicast_reply_locator_list: LocatorList::default(),
      have_timestamp: true,
      timestamp: Timestamp::TIME_INVALID,
    }
  }
}

#[cfg(test)]

mod tests {
  use super::*;
  use crate::messages::header::Header;
  use crate::speedy::{Writable, Readable};
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
  use crate::serialization::cdrSerializer::to_little_endian_binary;
  use serde::{Serialize, Deserialize};
  use crate::dds::writer::Writer;
  use mio_extras::channel as mio_channel;
  use crate::structure::{dds_cache::DDSCache, time::Timestamp};
  use std::sync::{RwLock, Arc};
  
  use crate::structure::topic_kind::TopicKind;
  use crate::dds::typedesc::TypeDesc;

  #[test]

  fn test_shapes_demo_message_deserialization() {
    // Data message should contain Shapetype values.
    // caprured with wireshark from shapes demo.
    // Udp packet with INFO_DST, INFO_TS, DATA, HEARTBEAT
    let udp_bits1: Vec<u8> = vec![
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x09, 0x01, 0x08, 0x00, 0x1a, 0x15, 0xf3, 0x5e, 0x00,
      0xcc, 0xfb, 0x13, 0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07,
      0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x69, 0x00, 0x00, 0x00, 0x17, 0x00,
      0x00, 0x00, 0x1e, 0x00, 0x00, 0x00, 0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00,
      0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x5b, 0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00,
    ];

    // this guid prefix is set here because exaple message target is this.
    let guiPrefix = GuidPrefix::new(vec![
      0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 0x31, 0xa2, 0x28, 0x20, 0x02, 0x8,
    ]);

    let mut message_receiver = MessageReceiver::new(guiPrefix);

    let entity = EntityId::createCustomEntityID([0, 0, 0], 7);
    let new_guid = GUID::new_with_prefix_and_id(guiPrefix, entity);
    
    new_guid.from_prefix(entity);
    let (send, _rec) = mio_channel::sync_channel::<()>(100);
    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));
    dds_cache.write().unwrap().add_new_topic(
      &"test".to_string(),
      TopicKind::NO_KEY,
      &TypeDesc::new("testi".to_string()),
    );
    let new_reader = Reader::new(
      new_guid,
      send,
      dds_cache,
      "test".to_string(),
    );

    // Skip for now+
    //new_reader.matched_writer_add(remote_writer_guid, mr_state);
    message_receiver.add_reader(new_reader);

    message_receiver.handle_user_msg(udp_bits1.clone());

    assert_eq!(message_receiver.submessage_count, 4);

    // this is not correct way to read history cache values but it serves as a test
    let sequenceNumbers =
      message_receiver.get_reader_history_cache_start_and_end_seq_num(new_guid.entityId);
    println!(
      "history change sequence number range: {:?}",
      sequenceNumbers
    );

    let a = message_receiver
      .get_reader_and_history_cache_change(new_guid.entityId, *sequenceNumbers.first().unwrap())
      .unwrap();
    println!("reader history chache DATA: {:?}", a.data());

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct ShapeType<'a> {
      color: &'a str,
      x: i32,
      y: i32,
      size: i32,
    }

    let copy_vec = (*a.data().clone()).to_vec();
    let deserializedShapeType: ShapeType = deserialize_from_little_endian(copy_vec).unwrap();
    println!("deserialized shapeType: {:?}", deserializedShapeType);
    assert_eq!(deserializedShapeType.color, "RED");

    println!();
    println!();
    println!();
    println!();
    println!();

    // now try to serialize same message

    let _serializedPayload = to_little_endian_binary(&deserializedShapeType);
    let (_dwcc_upload, hccc_download) = mio_channel::channel::<DDSData>();
    let mut _writerObject = Writer::new(
      GUID::new_with_prefix_and_id(guiPrefix, EntityId::createCustomEntityID([0, 0, 2], 2)),
      hccc_download,
      Arc::new(RwLock::new(DDSCache::new())),
      String::from("topicName1"),
    );
    let mut change = message_receiver.get_reader_and_history_cache_change_object(
      new_guid.entityId,
      *sequenceNumbers.first().unwrap(),
    );
    change.sequence_number = SequenceNumber::from(91);

    /*
    let _created_user_message = writerObject.write_user_msg(change);

    println!();
    println!();

    //assert_eq!(udp_bits1,createdUserMessage);
    //message_receiver.handle_user_msg(createdUserMessage);
    println!(
      "messageReceiver submessageCount: {:?}",
      message_receiver.submessage_count
    );
    */
  }

  #[test]
  fn mr_test_submsg_count() {
    // Udp packet with INFO_DST, INFO_TS, DATA, HEARTBEAT
    let udp_bits1: Vec<u8> = vec![
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x09, 0x01, 0x08, 0x00, 0x18, 0x15, 0xf3, 0x5e, 0x00,
      0x5c, 0xf0, 0x34, 0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07,
      0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x21, 0x00, 0x00, 0x00, 0x89, 0x00,
      0x00, 0x00, 0x1e, 0x00, 0x00, 0x00, 0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00,
      0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x43, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
    ];
    // Udp packet with INFO_DST, ACKNACK
    let udp_bits2: Vec<u8> = vec![
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x06, 0x03, 0x18, 0x00, 0x00, 0x00, 0x04, 0xc7, 0x00,
      0x00, 0x04, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x03, 0x00, 0x00, 0x00,
    ];

    let guid_new = GUID::new();
    let mut message_receiver = MessageReceiver::new(guid_new.guidPrefix);

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
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00,
    ];
    let guid_new = GUID::new();
    let mut message_receiver = MessageReceiver::new(guid_new.guidPrefix);
    assert!(message_receiver.handle_RTPS_header(&test_header));
  }
}
