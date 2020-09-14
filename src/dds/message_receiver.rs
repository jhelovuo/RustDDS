use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::vendor_id::VendorId;
use crate::messages::submessages::submessages::EntitySubmessage;
use crate::messages::submessages::submessages::*;
use crate::structure::guid::{GuidPrefix, GUID};
use crate::structure::entity::Entity;
use crate::structure::locator::{LocatorKind, LocatorList, Locator};
use crate::structure::time::{Time, Timestamp};
use crate::serialization::Message;
use crate::serialization::submessage::SubmessageBody;

use crate::dds::reader::Reader;
use crate::dds::ddsdata::DDSData;
use crate::structure::guid::EntityId;
use crate::{
  submessages::AckNack,
  structure::{
    cache_change::CacheChange,
    sequence_number::{SequenceNumber},
  },
};

use mio_extras::channel as mio_channel;

const RTPS_MESSAGE_HEADER_SIZE: usize = 20;

pub struct MessageReceiver {
  pub available_readers: Vec<Reader>,
  // GuidPrefix sent in this channel needs to be RTPSMessage source_guid_prefix. Writer needs this to locate RTPSReaderProxy if negative acknack.
  acknack_sender: mio_channel::Sender<(GuidPrefix, AckNack)>,

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
  pub fn new(
    participant_guid_prefix: GuidPrefix,
    acknack_sender: mio_channel::Sender<(GuidPrefix, AckNack)>,
  ) -> MessageReceiver {
    // could be passed in as a parameter
    let locator_kind = LocatorKind::LOCATOR_KIND_UDPv4;

    MessageReceiver {
      available_readers: Vec::new(),
      acknack_sender,
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
      .position(|r| r.get_guid() == old_reader_guid)
    {
      self.available_readers.remove(pos);
    }
  }

  fn get_reader(&mut self, reader_id: EntityId) -> Option<&mut Reader> {
    self
      .available_readers
      .iter_mut()
      .find(|r| r.get_entity_id() == reader_id)
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
      .find(|&r| r.get_entity_id() == reader_id)
      .unwrap();
    let a: Option<DDSData> = reader.get_history_cache_change_data(sequence_number);
    Some(a.unwrap())
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
      .find(|&r| r.get_entity_id() == reader_id)
      .unwrap();
    reader.get_history_cache_change(sequence_number).unwrap()
  }

  fn get_reader_history_cache_start_and_end_seq_num(
    &self,
    reader_id: EntityId,
  ) -> Vec<SequenceNumber> {
    let reader = self
      .available_readers
      .iter()
      .find(|&r| r.get_entity_id() == reader_id)
      .unwrap();
    reader.get_history_cache_sequence_start_and_end_numbers()
  }

  pub fn handle_discovery_msg(&mut self, msg: Vec<u8>) {
    // 9.6.2.2
    // The discovery message is just a data message. No need for the
    // messageReceiver to handle it any differently here?
    self.handle_user_msg(msg);
  }

  pub fn handle_user_msg(&mut self, msg_bytes: Vec<u8>) {
    self.reset();
    self.dest_guid_prefix = self.own_guid_prefix;

    // call Speedy reader
    let rtps_message = match Message::read_from_buffer(&msg_bytes) {
      Ok(m) => m,
      Err(speedy_err) => {
        println!("RTPS deserialize error {:?}", speedy_err);
        return;
      }
    };

    self.source_guid_prefix = rtps_message.header.guid_prefix;

    for submessage in rtps_message.submessages {
      match submessage.body {
        SubmessageBody::Interpreter(i) => self.handle_parsed_interpreter_submessage(i),
        SubmessageBody::Entity(e) => self.send_submessage(e),
      }
      self.submessage_count += 1;
    } // submessage loop
  }

  fn send_submessage(&mut self, submessage: EntitySubmessage) {
    if self.dest_guid_prefix != self.own_guid_prefix {
      println!("Messages are not for this participant?");
      println!("dest_guid_prefix: {:?}", self.dest_guid_prefix);
      println!("participant guid: {:?}", self.own_guid_prefix);
      return; // Wrong target received
    }

    let mr_state = self.give_message_receiver_info();
    match submessage {
      EntitySubmessage::Data(data, _) => {
        // If reader_id == ENTITYID_UNKNOWN, message should be sent to all matched readers
        if data.reader_id == EntityId::ENTITYID_UNKNOWN {
          for reader in self.available_readers.iter_mut() {
            reader.handle_data_msg(data.clone(), mr_state.clone());
          }
        } else {
          if let Some(target_reader) = self.get_reader(data.reader_id) {
            target_reader.handle_data_msg(data, mr_state);
          } else {
            println!(
              "MessageReceiver could not find corresponding Reader {:?}",
              data.reader_id
            );
          }
        }
      }
      EntitySubmessage::Heartbeat(heartbeat, flags) => {
        // If reader_id == ENTITYID_UNKNOWN, message should be sent to all matched readers
        if heartbeat.reader_id == EntityId::ENTITYID_UNKNOWN {
          for reader in self.available_readers.iter_mut() {
            reader.handle_heartbeat_msg(
              heartbeat.clone(),
              flags.contains(HEARTBEAT_Flags::Final),
              mr_state.clone(),
            );
          }
        } else {
          if let Some(target_reader) = self.get_reader(heartbeat.reader_id) {
            target_reader.handle_heartbeat_msg(
              heartbeat,
              flags.contains(HEARTBEAT_Flags::Final),
              mr_state,
            );
          } else {
            println!("MessageReceiver could not find corresponding Reader");
          }
        }
      }
      EntitySubmessage::Gap(gap, _flags) => {
        if let Some(target_reader) = self.get_reader(gap.reader_id) {
          target_reader.handle_gap_msg(gap, mr_state);
        } else {
          println!("MessageReceiver could not find corresponding Reader");
        }
      }
      EntitySubmessage::AckNack(ackNack, _) => {
        self
          .acknack_sender
          .send((self.source_guid_prefix, ackNack))
          .unwrap();
      }
      EntitySubmessage::DataFrag(datafrag, _) => {
        if let Some(target_reader) = self.get_reader(datafrag.reader_id) {
          target_reader.handle_datafrag_msg(datafrag, mr_state);
        } else {
          println!("MessageReceiver could not find corresponding Reader");
        }
      }
      EntitySubmessage::HeartbeatFrag(heartbeatfrag, _flags) => {
        // If reader_id == ENTITYID_UNKNOWN, message should be sent to all matched readers
        if heartbeatfrag.reader_id == EntityId::ENTITYID_UNKNOWN {
          for reader in self.available_readers.iter_mut() {
            reader.handle_heartbeatfrag_msg(heartbeatfrag.clone(), mr_state.clone());
          }
        } else {
          if let Some(target_reader) = self.get_reader(heartbeatfrag.reader_id) {
            target_reader.handle_heartbeatfrag_msg(heartbeatfrag, mr_state);
          } else {
            println!("MessageReceiver could not find corresponding Reader");
          }
        }
      }
      EntitySubmessage::NackFrag(_, _) => {}
    }
  }

  fn handle_parsed_interpreter_submessage(&mut self, interp_subm: InterpreterSubmessage)
  // no return value, just change state of self.
  {
    match interp_subm {
      InterpreterSubmessage::InfoTimestamp(ts_struct, flags) => {
        if flags.contains(INFOTIMESTAMP_Flags::Invalidate) {
          self.have_timestamp = true;
          self.timestamp = ts_struct.timestamp;
        }
      }
      InterpreterSubmessage::InfoSource(info_src, _flags) => {
        self.source_guid_prefix = info_src.guid_prefix;
        self.source_version = info_src.protocol_version;
        self.source_vendor_id = info_src.vendor_id;
        self.unicast_reply_locator_list.clear(); // Or invalid?
        self.multicast_reply_locator_list.clear(); // Or invalid?
        self.have_timestamp = false;
      }
      InterpreterSubmessage::InfoReply(info_reply, flags) => {
        self.unicast_reply_locator_list = info_reply.unicast_locator_list;
        if flags.contains(INFOREPLY_Flags::Multicast) {
          self.multicast_reply_locator_list = info_reply
            .multicast_locator_list
            .expect("InfoReply flag indicates multicast locator is present but none found.");
        // TODO: Convert the above error to warning only.
        } else {
          self.multicast_reply_locator_list.clear();
        }
      }
      InterpreterSubmessage::InfoDestination(info_dest, _flags) => {
        if info_dest.guid_prefix != GUID::GUID_UNKNOWN.guidPrefix {
          self.dest_guid_prefix = info_dest.guid_prefix;
        } else {
          self.dest_guid_prefix = self.own_guid_prefix;
        }
      }
    }
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
  use crate::serialization::cdrSerializer::to_bytes;
  use byteorder::LittleEndian;
  use serde::{Serialize, Deserialize};
  use crate::dds::writer::Writer;
  use mio_extras::channel as mio_channel;
  use crate::structure::dds_cache::DDSCache;
  use std::sync::{RwLock, Arc};

  use crate::structure::topic_kind::TopicKind;
  use crate::dds::{qos::QosPolicies, typedesc::TypeDesc};

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

    let (acknack_sender, _acknack_reciever) = mio_channel::channel::<(GuidPrefix, AckNack)>();
    let mut message_receiver = MessageReceiver::new(guiPrefix, acknack_sender);

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
    let new_reader = Reader::new(new_guid, send, dds_cache, "test".to_string());

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
    struct ShapeType {
      color: String,
      x: i32,
      y: i32,
      size: i32,
    }

    let deserializedShapeType: ShapeType = deserialize_from_little_endian(&a.data()).unwrap();
    println!("deserialized shapeType: {:?}", deserializedShapeType);
    assert_eq!(deserializedShapeType.color, "RED");

    println!();
    println!();
    println!();
    println!();
    println!();

    // now try to serialize same message

    let _serializedPayload = to_bytes::<ShapeType, LittleEndian>(&deserializedShapeType);
    let (_dwcc_upload, hccc_download) = mio_channel::channel::<DDSData>();
    let mut _writerObject = Writer::new(
      GUID::new_with_prefix_and_id(guiPrefix, EntityId::createCustomEntityID([0, 0, 2], 2)),
      hccc_download,
      Arc::new(RwLock::new(DDSCache::new())),
      String::from("topicName1"),
      QosPolicies::qos_none(),
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
    let (acknack_sender, _acknack_reciever) = mio_channel::channel::<(GuidPrefix, AckNack)>();
    let mut message_receiver = MessageReceiver::new(guid_new.guidPrefix, acknack_sender);

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
}
