use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::vendor_id::VendorId;
use crate::messages::submessages::submessages::EntitySubmessage;
use crate::messages::submessages::submessages::*;
use crate::structure::guid::{GuidPrefix, GUID};
use crate::structure::entity::RTPSEntity;
use crate::structure::locator::{LocatorKind, LocatorList, Locator};
use crate::structure::time::Timestamp;
use crate::serialization::Message;
use crate::serialization::submessage::SubmessageBody;

use crate::dds::reader::Reader;
use crate::structure::guid::EntityId;

#[cfg(test)] use crate::dds::ddsdata::DDSData;
#[cfg(test)] use crate::structure::cache_change::CacheChange;
#[cfg(test)] use crate::structure::sequence_number::{SequenceNumber};

use mio_extras::channel as mio_channel;
use log::{debug, warn, trace, info};
use bytes::Bytes;

use std::collections::{BTreeMap, btree_map::Entry};


const RTPS_MESSAGE_HEADER_SIZE: usize = 20;

/// MessageReceiver is the submessage sequence interpreter described in 
/// RTPS spec v2.3 Section 8.3.4 "The RTPS Message Receiver".
/// It calls the message/submessage deserializers to parse the sequence of submessages.
/// Then it processes the instructions in the Interpreter SUbmessages and forwards data in
/// Enity Submessages to the appropriate Entities. (See RTPS spec Section 8.3.7)

pub(crate) struct MessageReceiver {
  pub available_readers: BTreeMap<EntityId,Reader>,
  // GuidPrefix sent in this channel needs to be RTPSMessage source_guid_prefix. Writer needs this to locate RTPSReaderProxy if negative acknack.
  acknack_sender: mio_channel::SyncSender<(GuidPrefix, AckSubmessage)>,

  own_guid_prefix: GuidPrefix,
  pub source_version: ProtocolVersion,
  pub source_vendor_id: VendorId,
  pub source_guid_prefix: GuidPrefix,
  pub dest_guid_prefix: GuidPrefix,
  pub unicast_reply_locator_list: LocatorList,
  pub multicast_reply_locator_list: LocatorList,
  pub timestamp: Option<Timestamp>,

  pos: usize,
  pub submessage_count: usize,
}

impl MessageReceiver {
  pub fn new(
    participant_guid_prefix: GuidPrefix,
    acknack_sender: mio_channel::SyncSender<(GuidPrefix, AckSubmessage)>,
  ) -> MessageReceiver {
    // could be passed in as a parameter
    let locator_kind = LocatorKind::LOCATOR_KIND_UDPv4;

    MessageReceiver {
      available_readers: BTreeMap::new(),
      acknack_sender,
      own_guid_prefix: participant_guid_prefix,

      source_version: ProtocolVersion::THIS_IMPLEMENTATION,
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
      timestamp: None,

      pos: 0,
      submessage_count: 0,
    }
  }

  pub fn reset(&mut self) {
    self.source_version = ProtocolVersion::THIS_IMPLEMENTATION;
    self.source_vendor_id = VendorId::VENDOR_UNKNOWN;
    self.source_guid_prefix = GuidPrefix::GUIDPREFIX_UNKNOWN;
    self.dest_guid_prefix = GuidPrefix::GUIDPREFIX_UNKNOWN;
    self.unicast_reply_locator_list.clear();
    self.multicast_reply_locator_list.clear();
    self.timestamp = None;

    self.pos = 0;
    self.submessage_count = 0;
  }

  fn give_message_receiver_info(&self) -> MessageReceiverState {
    MessageReceiverState {
      //own_guid_prefix: self.own_guid_prefix,
      source_guid_prefix: self.source_guid_prefix,
      unicast_reply_locator_list: self.unicast_reply_locator_list.clone(),
      multicast_reply_locator_list: self.multicast_reply_locator_list.clone(),
      timestamp: self.timestamp,
    }
  }

  pub fn add_reader(&mut self, new_reader: Reader) {
    let eid = new_reader.get_guid().entityId;
    match self.available_readers.entry( eid ) {
      Entry::Occupied( _ ) => warn!("Already have Reader {:?} - not adding.",eid) ,
      e => { e.or_insert(new_reader); }
    }
  }

  pub fn remove_reader(&mut self, old_reader_guid: GUID) -> Option<Reader> {
    self.available_readers.remove( &old_reader_guid.entityId )
  }

  pub fn get_reader_mut(&mut self, reader_id: EntityId) -> Option<&mut Reader> {
    self.available_readers.get_mut( &reader_id )
  }

  // use for test and debugging only
  #[cfg(test)]
  fn get_reader_and_history_cache_change(&self, reader_id: EntityId, 
    sequence_number: SequenceNumber,
  ) -> Option<DDSData> {
    Some (self.available_readers.get( &reader_id ).unwrap()
      .get_history_cache_change_data(sequence_number)
      .unwrap() )
  }

  // use for test and debugging only
  #[cfg(test)]
  fn get_reader_and_history_cache_change_object(
    &self,
    reader_id: EntityId,
    sequence_number: SequenceNumber,
  ) -> CacheChange {
    self.available_readers.get( &reader_id ).unwrap()
      .get_history_cache_change(sequence_number)
      .unwrap()
  }

  #[cfg(test)]
  fn get_reader_history_cache_start_and_end_seq_num( &self, reader_id: EntityId ) 
      -> Vec<SequenceNumber> {
    self.available_readers.get( &reader_id ).unwrap()
      .get_history_cache_sequence_start_and_end_numbers()
  }

  // pub fn handle_discovery_msg(&mut self, msg: Bytes) {
  //   // 9.6.2.2
  //   // The discovery message is just a data message. No need for the
  //   // messageReceiver to handle it any differently here?
  //   self.handle_user_msg(msg);
  // }

  pub fn handle_received_packet(&mut self, msg_bytes: Bytes) {
    // Check for RTPS ping message. At least RTI implementation sends these.
    // What should we do with them? The spec does not say.
    if msg_bytes.len() < RTPS_MESSAGE_HEADER_SIZE {
      if msg_bytes.len() >= 16 && msg_bytes[0..4] == b"RTPS"[..] && msg_bytes[9..16] == b"DDSPING"[..] {
        // TODO: Add some sensible ping message handling here. 
        info!("Received RTPS PING. Do not know how to respond.");
        debug!("Data was {:?}",&msg_bytes);
        return
      } else {
        warn!("Message is shorter than header. Cannot deserialize.");
        debug!("Data was {:?}",&msg_bytes);
        return
      }
    }

    // call Speedy reader
    // Bytes .clone() is cheap, so no worries
    let rtps_message = match Message::read_from_buffer(msg_bytes.clone()) {
      Ok(m) => m,
      Err(speedy_err) => {
        warn!("RTPS deserialize error {:?}", speedy_err);
        debug!("Data was {:?}",msg_bytes);
        return
      }
    };

    // And process message
    self.handle_parsed_message(rtps_message)
  }

  // This is also called directly from dp_event_loop in case of loopback messages.
  pub fn handle_parsed_message(&mut self, rtps_message: Message)
  {
    self.reset();
    self.dest_guid_prefix = self.own_guid_prefix;
    self.source_guid_prefix = rtps_message.header.guid_prefix;

    for submessage in rtps_message.submessages {
      match submessage.body {
        SubmessageBody::Interpreter(i) => self.handle_interpreter_submessage(i),
        SubmessageBody::Entity(e) => self.handle_entity_submessage(e),
      }
      self.submessage_count += 1;
    } // submessage loop
  }

  fn handle_entity_submessage(&mut self, submessage: EntitySubmessage) {
    if self.dest_guid_prefix != self.own_guid_prefix 
        && self.dest_guid_prefix != GuidPrefix::GUIDPREFIX_UNKNOWN {
      debug!("Message is not for this participant. Dropping. dest_guid_prefix={:?} participant guid={:?}", 
        self.dest_guid_prefix, self.own_guid_prefix);
      return 
    }

    let mr_state = self.give_message_receiver_info();
    match submessage {
      EntitySubmessage::Data(data, data_flags) => {
        // If reader_id == ENTITYID_UNKNOWN, message should be sent to all matched readers
        if data.reader_id == EntityId::ENTITYID_UNKNOWN {
          trace!("handle_entity_submessage DATA from unknown. writer_id = {:?}", &data.writer_id);
          for reader in self
            .available_readers
            .values_mut()
            .filter(|r| r.contains_writer(data.writer_id) 
                        // exception: discovery prococol reader must read from unkonwn discovery protocol writers
                        // TODO: This logic here is uglyish. Can we just inject a presupposed writer (proxy)
                        // to the built-in reader as it is created?
                        || (data.writer_id == EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER
                            &&
                            r.get_entity_id() == EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER
                            )
                      )
          {
            debug!("handle_entity_submessage DATA from unknown handling in {:?}",&reader);
            reader.handle_data_msg(data.clone(), data_flags, mr_state.clone());
          }
        } else {
          if let Some(target_reader) = self.get_reader_mut(data.reader_id) {
            target_reader.handle_data_msg(data, data_flags, mr_state);
          }
        }
      }
      EntitySubmessage::Heartbeat(heartbeat, flags) => {
        // If reader_id == ENTITYID_UNKNOWN, message should be sent to all matched readers
        if heartbeat.reader_id == EntityId::ENTITYID_UNKNOWN {
          for reader in self
            .available_readers
            .values_mut()
            .filter(|p| p.contains_writer(heartbeat.writer_id))
          {
            reader.handle_heartbeat_msg(
              heartbeat.clone(),
              flags.contains(HEARTBEAT_Flags::Final),
              mr_state.clone(),
            );
          }
        } else {
          if let Some(target_reader) = self.get_reader_mut(heartbeat.reader_id) {
            target_reader.handle_heartbeat_msg(
              heartbeat,
              flags.contains(HEARTBEAT_Flags::Final),
              mr_state,
            );
          }
        }
      }
      EntitySubmessage::Gap(gap, _flags) => {
        if let Some(target_reader) = self.get_reader_mut(gap.reader_id) {
          target_reader.handle_gap_msg(gap, mr_state);
        }
      }
      EntitySubmessage::AckNack(acknack, _) => {
        match self.acknack_sender.send((self.source_guid_prefix, 
            AckSubmessage::AckNack_Variant(acknack))) {
          Ok(_) => (),
          Err(e) => warn!("Failed to send AckNack. {:?}", e),
        }
      }
      EntitySubmessage::DataFrag(datafrag, flags) => {
        if let Some(target_reader) = self.get_reader_mut(datafrag.reader_id) {
          target_reader.handle_datafrag_msg(datafrag, flags, mr_state, );
        }
      }
      EntitySubmessage::HeartbeatFrag(heartbeatfrag, _flags) => {
        // If reader_id == ENTITYID_UNKNOWN, message should be sent to all matched readers
        if heartbeatfrag.reader_id == EntityId::ENTITYID_UNKNOWN {
          for reader in self
            .available_readers
            .values_mut()
            .filter(|p| p.contains_writer(heartbeatfrag.writer_id))
          {
            reader.handle_heartbeatfrag_msg(heartbeatfrag.clone(), mr_state.clone());
          }
        } else {
          if let Some(target_reader) = self.get_reader_mut(heartbeatfrag.reader_id) {
            target_reader.handle_heartbeatfrag_msg(heartbeatfrag, mr_state);
          }
        }
      }
      EntitySubmessage::NackFrag(_, _) => {}
    }
  }

  fn handle_interpreter_submessage(&mut self, interp_subm: InterpreterSubmessage)
  // no return value, just change state of self.
  {
    match interp_subm {
      InterpreterSubmessage::InfoTimestamp(ts_struct, _flags) => {
        // flags value was used already when parsing timestamp into an Option
        self.timestamp = ts_struct.timestamp;
      }
      InterpreterSubmessage::InfoSource(info_src, _flags) => {
        self.source_guid_prefix = info_src.guid_prefix;
        self.source_version = info_src.protocol_version;
        self.source_vendor_id = info_src.vendor_id;
        self.unicast_reply_locator_list.clear(); // Or invalid?
        self.multicast_reply_locator_list.clear(); // Or invalid?
        self.timestamp = None;
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

  pub fn notify_data_to_readers(&self, readers: Vec<EntityId>) {
    for eid in readers {
      self.available_readers.get(&eid)
        .map( |r| r.notify_cache_change() );
    }
  }

  // sends 0 seqnum acknacks for those writer that haven't had any action
  pub fn send_preemptive_acknacks(&mut self) {
    for reader in self.available_readers.values_mut() {
      reader.send_preemptive_acknacks()
    }
  }
} // impl messageReceiver

#[derive(Debug, Clone)]
pub struct MessageReceiverState {
  //pub own_guid_prefix: GuidPrefix,
  pub source_guid_prefix: GuidPrefix,
  pub unicast_reply_locator_list: LocatorList,
  pub multicast_reply_locator_list: LocatorList,
  pub timestamp: Option<Timestamp>,
}

impl Default for MessageReceiverState {
  fn default() -> Self {
    Self {
      //own_guid_prefix: GuidPrefix::default(),
      source_guid_prefix: GuidPrefix::default(),
      unicast_reply_locator_list: LocatorList::default(),
      multicast_reply_locator_list: LocatorList::default(),
      timestamp: Some(Timestamp::TIME_INVALID),
    }
  }
}

#[cfg(test)]

mod tests {
  use crate::structure::sequence_number::SequenceNumber;
use super::*;
  use crate::{
    dds::writer::WriterCommand, messages::header::Header,
    dds::with_key::datareader::ReaderCommand,
  };
  use crate::dds::reader::ReaderIngredients;
  use crate::dds::writer::WriterIngredients;
  use crate::network::udp_sender::UDPSender;
  use crate::dds::statusevents::DataReaderStatus;
  use crate::speedy::{Writable, Readable};
  use crate::serialization::cdr_deserializer::deserialize_from_little_endian;
  use crate::serialization::cdr_serializer::to_bytes;
  use byteorder::LittleEndian;
  use log::info;
  use serde::{Serialize, Deserialize};
  use crate::dds::writer::Writer;
  use mio_extras::channel as mio_channel;
  use crate::structure::dds_cache::DDSCache;
  use std::sync::{RwLock, Arc};
  use std::rc::Rc;

  use crate::structure::topic_kind::TopicKind;
  use crate::structure::guid::EntityKind;
  use crate::dds::{qos::QosPolicies, typedesc::TypeDesc};

  #[test]

  fn test_shapes_demo_message_deserialization() {
    // Data message should contain Shapetype values.
    // caprured with wireshark from shapes demo.
    // Udp packet with INFO_DST, INFO_TS, DATA, HEARTBEAT
    let udp_bits1 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x09, 0x01, 0x08, 0x00, 0x1a, 0x15, 0xf3, 0x5e, 0x00,
      0xcc, 0xfb, 0x13, 0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07,
      0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x69, 0x00, 0x00, 0x00, 0x17, 0x00,
      0x00, 0x00, 0x1e, 0x00, 0x00, 0x00, 0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00,
      0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x5b, 0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00,
    ]);

    // this guid prefix is set here because exaple message target is this.
    let guiPrefix = GuidPrefix::new(&[
      0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 0x31, 0xa2, 0x28, 0x20, 0x02, 0x8,
    ]);

    let (acknack_sender, _acknack_reciever) =
      mio_channel::sync_channel::<(GuidPrefix, AckSubmessage)>(10);
    let mut message_receiver = MessageReceiver::new(guiPrefix, acknack_sender);

    let entity = EntityId::createCustomEntityID([0, 0, 0], EntityKind::READER_WITH_KEY_USER_DEFINED);
    let new_guid = GUID::new_with_prefix_and_id(guiPrefix, entity);

    new_guid.from_prefix(entity);
    let (send, _rec) = mio_channel::sync_channel::<()>(100);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<DataReaderStatus>(100);
    let (_reader_commander, reader_command_receiver) =
      mio_extras::channel::sync_channel::<ReaderCommand>(100);

    let dds_cache = Arc::new(RwLock::new(DDSCache::new()));
    dds_cache.write().unwrap().add_new_topic(
      &"test".to_string(),
      TopicKind::NoKey,
      TypeDesc::new("testi"),
    );
    let reader_ing = ReaderIngredients {
      guid: new_guid,
      notification_sender: send,
      status_sender,
      topic_name: "test".to_string(),
      qos_policy: QosPolicies::qos_none(),
      data_reader_command_receiver: reader_command_receiver,
    };

    let new_reader = Reader::new(reader_ing, dds_cache,
      Rc::new(UDPSender::new_with_random_port().unwrap()));

    // Skip for now+
    //new_reader.matched_writer_add(remote_writer_guid, mr_state);
    message_receiver.add_reader(new_reader);

    message_receiver.handle_received_packet(udp_bits1.clone());

    assert_eq!(message_receiver.submessage_count, 4);

    // this is not correct way to read history cache values but it serves as a test
    let sequenceNumbers =
      message_receiver.get_reader_history_cache_start_and_end_seq_num(new_guid.entityId);
    info!(
      "history change sequence number range: {:?}",
      sequenceNumbers
    );

    let a = message_receiver
      .get_reader_and_history_cache_change(new_guid.entityId, *sequenceNumbers.first().unwrap())
      .unwrap();
    info!("reader history chache DATA: {:?}", a.data());

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct ShapeType {
      color: String,
      x: i32,
      y: i32,
      size: i32,
    }

    let deserializedShapeType: ShapeType = deserialize_from_little_endian(&a.data().unwrap()).unwrap();
    info!("deserialized shapeType: {:?}", deserializedShapeType);
    assert_eq!(deserializedShapeType.color, "RED");

    // now try to serialize same message

    let _serializedPayload = to_bytes::<ShapeType, LittleEndian>(&deserializedShapeType);
    let (_dwcc_upload, hccc_download) = mio_channel::channel::<WriterCommand>();
    let (status_sender, _status_receiver) = mio_channel::sync_channel(10);

    let writer_ing = WriterIngredients {
      guid: GUID::new_with_prefix_and_id(guiPrefix, 
          EntityId::createCustomEntityID([0, 0, 2], EntityKind::WRITER_WITH_KEY_USER_DEFINED)),
      writer_command_receiver: hccc_download,
      topic_name: String::from("topicName1"),
      qos_policies: QosPolicies::qos_none(),
      status_sender,
    };

    let mut _writerObject = Writer::new(
      writer_ing,
      Arc::new(RwLock::new(DDSCache::new())),
      Rc::new(UDPSender::new_with_random_port().unwrap())
    );
    let mut change = message_receiver.get_reader_and_history_cache_change_object(
      new_guid.entityId,
      *sequenceNumbers.first().unwrap(),
    );
    change.sequence_number = SequenceNumber::from(91);
  }

  #[test]
  fn mr_test_submsg_count() {
    // Udp packet with INFO_DST, INFO_TS, DATA, HEARTBEAT
    let udp_bits1 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x09, 0x01, 0x08, 0x00, 0x18, 0x15, 0xf3, 0x5e, 0x00,
      0x5c, 0xf0, 0x34, 0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07,
      0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x21, 0x00, 0x00, 0x00, 0x89, 0x00,
      0x00, 0x00, 0x1e, 0x00, 0x00, 0x00, 0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00,
      0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x43, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
    ]);
    // Udp packet with INFO_DST, ACKNACK
    let udp_bits2 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x06, 0x03, 0x18, 0x00, 0x00, 0x00, 0x04, 0xc7, 0x00,
      0x00, 0x04, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x03, 0x00, 0x00, 0x00,
    ]);

    let guid_new = GUID::default();
    let (acknack_sender, _acknack_reciever) =
      mio_channel::sync_channel::<(GuidPrefix, AckSubmessage)>(10);
    let mut message_receiver = MessageReceiver::new(guid_new.guidPrefix, acknack_sender);

    message_receiver.handle_received_packet(udp_bits1);
    assert_eq!(message_receiver.submessage_count, 4);

    message_receiver.handle_received_packet(udp_bits2);
    assert_eq!(message_receiver.submessage_count, 2);
  }

  #[test]
  fn mr_test_header() {
    let guid_new = GUID::default();
    let header = Header::new(guid_new.guidPrefix);

    let bytes = header.write_to_vec().unwrap();
    let new_header = Header::read_from_buffer(&bytes).unwrap();
    assert_eq!(header, new_header);
  }
}
