use crate::structure::history_cache::HistoryCache;
use crate::messages::submessages::data::Data;
use crate::messages::submessages::info_destination::InfoDestination;
use crate::messages::submessages::info_timestamp::InfoTimestamp;
use crate::structure::time::Timestamp;
use byteorder::{ByteOrder, LittleEndian};
use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::{header::Header, vendor_id::VendorId, protocol_id::ProtocolId};
use crate::structure::guid::{GuidPrefix, EntityId};
use crate::structure::sequence_number::SequenceNumber;
use crate::{
  submessages::{
    Heartbeat, InfoSource, SubmessageHeader, SubmessageKind, SubmessageFlag, InterpreterSubmessage,
    EntitySubmessage,
  },
  structure::cache_change::CacheChange,
  serialization::{SubMessage, Message},
};
use speedy::{Writable, Endianness};
use time::Timespec;
use time::get_time;

use mio_extras::channel as mio_channel;
use mio::Token;

use crate::dds::ddsdata::DDSData;
use crate::structure::entity::{Entity, EntityAttributes};
use crate::structure::guid::GUID;

pub struct Writer {
  history_cache: HistoryCache,
  pub submessage_count: usize,

  source_version: ProtocolVersion,
  source_vendor_id: VendorId,
  pub source_guid_prefix: GuidPrefix,
  pub dest_guid_prefix: GuidPrefix,

  pub endianness: Endianness,

  heartbeatMessageCounter: i32,

  entity_attributes: EntityAttributes,
  cache_change_receiver: mio_channel::Receiver<DDSData>,
}

impl Writer {
  pub fn new(guid: GUID, cache_change_receiver: mio_channel::Receiver<DDSData>) -> Writer {
    let entity_attributes = EntityAttributes::new(guid);
    Writer {
      history_cache: HistoryCache::new(),
      submessage_count: 0,
      source_version: ProtocolVersion::PROTOCOLVERSION_2_3,
      source_vendor_id: VendorId::VENDOR_UNKNOWN,
      source_guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
      dest_guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
      endianness: Endianness::LittleEndian,
      heartbeatMessageCounter: 0,
      entity_attributes,
      cache_change_receiver,
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

  fn increase_heartbeat_counter(&mut self) {
    self.heartbeatMessageCounter = self.heartbeatMessageCounter + 1;
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

  pub fn write_SRC_message(&self, message_buffer: &mut Vec<u8>) {
    let source = InfoSource {
      protocol_version: ProtocolVersion {
        major: self.source_version.major,
        minor: self.source_version.minor,
      },
      vendor_id: VendorId {
        vendorId: self.source_vendor_id.vendorId,
      },
      guid_prefix: self.source_guid_prefix,
    };
    message_buffer.append(&mut source.write_to_vec_with_ctx(self.endianness).unwrap());
  }

  pub fn get_DATA_msg_from_cache_change(&self, change: CacheChange) -> SubMessage {
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

  pub fn get_heartbeat_msg(&mut self) -> SubMessage {
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
      count: self.heartbeatMessageCounter,
    };
    self.increase_heartbeat_counter();
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

  pub fn act_to_cache_chage() {
    todo!();
  }

  pub fn write_user_msg(&mut self, change: CacheChange) -> Vec<u8> {
    let mut message: Vec<u8> = vec![];

    let mut RTPSMessage: Message = Message::new();
    RTPSMessage.set_header(self.create_message_header());

    // TODO get Destination from recieving reader list ???
    let destiny = InfoDestination {
      guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
    };

    let size = destiny
      .write_to_vec_with_ctx(self.endianness)
      .unwrap()
      .len();
    let DSTSubmessage: SubMessage = SubMessage {
      header: self.create_submessage_header(SubmessageKind::INFO_DST, size as u16),
      submessage: None,
      intepreterSubmessage: Some(InterpreterSubmessage::InfoDestination(destiny)),
    };
    RTPSMessage.add_submessage(DSTSubmessage);
    RTPSMessage.add_submessage(self.get_TS_submessage());
    RTPSMessage.add_submessage(self.get_DATA_msg_from_cache_change(change));
    RTPSMessage.add_submessage(self.get_heartbeat_msg());

    println!("RTPS message: {:?}", RTPSMessage);
    message.append(&mut RTPSMessage.write_to_vec_with_ctx(self.endianness).unwrap());

    println!("serialized RTPS message: {:?}", message);
    return message;
  }

  pub fn make_cache_change() {
    todo!();
  }

  fn get_list_of_recieving_readers() {
    todo!();
  }

  fn add_recieving_reader() {
    todo!();
  }

  fn remove_recieving_reader() {
    todo!();
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
