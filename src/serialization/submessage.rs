use crate::messages::submessages::submessage_header::SubmessageHeader;
use crate::messages::submessages::submessage_kind::SubmessageKind;
use crate::messages::submessages::submessage::EntitySubmessage;

use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::messages::submessages::submessage_elements::parameter_list::ParameterList;

use crate::structure::guid::EntityId;
use crate::structure::sequence_number::SequenceNumber;

use crate::messages::submessages::ack_nack::*;
use crate::messages::submessages::gap::*;
use crate::messages::submessages::heartbeat::*;
use crate::messages::submessages::heartbeat_frag::*;
use crate::messages::submessages::nack_frag::*;
use crate::messages::submessages::data::*;
use crate::messages::submessages::data_frag::*;
use crate::messages::fragment_number::FragmentNumber;
use speedy::{Context, Readable, Writable, Endianness};

#[derive(Debug, PartialEq)]
pub struct SubMessage {
  pub header: SubmessageHeader,
  pub submessage: EntitySubmessage,
}

impl<'a> SubMessage
where
// T: Readable<'a, Endianness> + Writable<Endianness>,
{
  pub fn deserialize_header(context: Endianness, buffer: &'a [u8]
  ) -> Option<SubmessageHeader> {
    match SubmessageHeader::read_from_buffer_with_ctx(context, buffer) {
      Ok(T) => Some(T),
      Err(_) => None
    }
  }

  pub fn deserialize_msg(
    msgheader: &SubmessageHeader,
    context: Endianness,
    buffer: &'a [u8],
  ) -> Option<EntitySubmessage> {
    match msgheader.submessage_id {
      SubmessageKind::ACKNACK => Some(EntitySubmessage::AckNack(
        AckNack::read_from_buffer_with_ctx(context, buffer).unwrap(),
        msgheader.flags.clone(),
      )),
      SubmessageKind::HEARTBEAT => Some(EntitySubmessage::Heartbeat(
        Heartbeat::read_from_buffer_with_ctx(context, buffer).unwrap(),
        msgheader.flags.clone(),
      )),
      SubmessageKind::GAP => Some(EntitySubmessage::Gap(
        Gap::read_from_buffer_with_ctx(context, buffer).unwrap(),
      )),
      SubmessageKind::NACK_FRAG => Some(EntitySubmessage::NackFrag(
        NackFrag::read_from_buffer_with_ctx(context, buffer).unwrap(),
      )),
      SubmessageKind::HEARTBEAT_FRAG => Some(EntitySubmessage::HeartbeatFrag(
        HeartbeatFrag::read_from_buffer_with_ctx(context, buffer).unwrap(),
      )),
      SubmessageKind::DATA => 
        match SubMessage::deserialize_data(&msgheader, &context, &buffer){
          Some(T) => Some(EntitySubmessage::Data(
            T,
            msgheader.flags.clone(),
          )),
          None => None,
      },
      SubmessageKind::DATA_FRAG => 
        match SubMessage::deserialize_data_frag(&msgheader, &context, &buffer){
          Some(T) => Some(EntitySubmessage::DataFrag(
            T,
            msgheader.flags.clone(),
          )),
          None => None,
      },
      _ => None, // Unknown id, must be ignored
    }
  }

  fn deserialize_data(
    msgheader: &SubmessageHeader,
    _context: &Endianness,
    buffer: &'a [u8],
  ) -> Option<Data> {
    let mut pos: usize = 0;
    // TODO Check flag locations..?
    //let endianness_flag = msgheader.flags.is_flag_set(3);
    let inline_qoS_flag = msgheader.flags.is_flag_set(2);
    let data_flag = msgheader.flags.is_flag_set(1);
    let key_flag = msgheader.flags.is_flag_set(0);
    if key_flag && data_flag {
      return None;
    }
    pos += 2; // Ignore extra flags for now

    let octets_to_inline_qos = u16::read_from_buffer(
      &buffer[pos..pos+2]).unwrap();
    pos += 2;

    let reader_id = EntityId::read_from_buffer(
      &buffer[pos..pos+4]).unwrap();
    pos += 4;
    let writer_id = EntityId::read_from_buffer(
      &buffer[pos..pos+4]).unwrap();
    pos += 4;
    let writer_sn = SequenceNumber::read_from_buffer(
      &buffer[pos..pos+8]).unwrap();
    pos += 8;

    if pos != octets_to_inline_qos as usize + 4 {
      pos = octets_to_inline_qos as usize + 4; // Ignore 
    }
    let mut inline_qos = ParameterList{parameters: vec![]};
    if inline_qoS_flag {
      let inline_qos_size: usize = 4; // Is it this for sure??
      inline_qos = ParameterList::read_from_buffer(
        &buffer[pos..pos+inline_qos_size]).unwrap();
        pos += inline_qos_size;
    } 
    let mut serialized_payload = SerializedPayload::new();
    if data_flag || key_flag {
      let rep_identifier = u16::read_from_buffer(&buffer[pos..pos+2]).unwrap();
      pos += 2;
      let rep_options = u16::read_from_buffer(&buffer[pos..pos+2]).unwrap();
      pos += 2;

      let payload_size: usize = msgheader.submessage_length as usize - pos;

      let vec_value = Vec::read_from_buffer(
        &buffer[pos..pos+payload_size]
      ).unwrap();

      serialized_payload = SerializedPayload{
        representation_identifier: rep_identifier,
        representation_options: rep_options,
        value: vec_value,
      }
    }
    Some(Data{
      reader_id,
      writer_id,
      writer_sn,
      inline_qos,
      serialized_payload,
    })
  }

  fn deserialize_data_frag(
    msgheader: &SubmessageHeader,
    _context: &Endianness,
    buffer: &'a [u8],
  ) -> Option<DataFrag> {
    let mut pos: usize = 0;
    // TODO Check flag locations..?
    //let endianness_flag = msgheader.flags.is_flag_set(3);
    let inline_qoS_flag = msgheader.flags.is_flag_set(2);
    //let non_standar_payload_flag = msgheader.flags.is_flag_set(1);
    let key_flag = msgheader.flags.is_flag_set(0);

    pos += 2; // Ignore extra flags for now

    let octets_to_inline_qos = u16::read_from_buffer(
      &buffer[pos..pos+2]).unwrap();
    pos += 2;

    let reader_id = EntityId::read_from_buffer(
      &buffer[pos..pos+4]).unwrap();
    pos += 4;
    let writer_id = EntityId::read_from_buffer(
      &buffer[pos..pos+4]).unwrap();
    pos += 4;
    let writer_sn = SequenceNumber::read_from_buffer(
      &buffer[pos..pos+8]).unwrap();
    pos += 8;
    let fragment_starting_num = FragmentNumber::read_from_buffer(
      &buffer[pos..pos+4]).unwrap();
    pos += 4;
    let fragments_in_submessage = u16::read_from_buffer(
      &buffer[pos..pos+2]).unwrap();
    pos += 2;
    let fragment_size = u16::read_from_buffer(
      &buffer[pos..pos+2]).unwrap();
    pos += 2;
    let data_size = u32::read_from_buffer(
      &buffer[pos..pos+4]).unwrap();
    pos += 4;

    if pos != octets_to_inline_qos as usize + 4 {
      pos = octets_to_inline_qos as usize + 4; // Ignore 
    }

    let mut inline_qos = ParameterList{parameters: vec![]};
    if inline_qoS_flag {
      let inline_qos_size: usize = 4; // Is it this for sure??
      inline_qos = ParameterList::read_from_buffer(
        &buffer[pos..pos+inline_qos_size]).unwrap();
        pos += inline_qos_size;
    } 
    let mut serialized_payload = SerializedPayload::new();
    if !key_flag {
      let rep_identifier = u16::read_from_buffer(&buffer[pos..pos+2]).unwrap();
      pos += 2;
      let rep_options = u16::read_from_buffer(&buffer[pos..pos+2]).unwrap();
      pos += 2;

      let payload_size: usize = msgheader.submessage_length as usize - pos;

      let vec_value = Vec::read_from_buffer(
        &buffer[pos..pos+payload_size]
      ).unwrap();

      serialized_payload = SerializedPayload{
        representation_identifier: rep_identifier,
        representation_options: rep_options,
        value: vec_value,
      }
    }
    Some(DataFrag{
      reader_id,
      writer_id,
      writer_sn,
      fragment_starting_num,
      fragments_in_submessage,
      data_size,
      fragment_size,
      inline_qos,
      serialized_payload,
    })
  }

  
  pub fn serialize_header(self) -> Vec<u8> {
    let buffer = self.header.write_to_vec_with_ctx(
      Endianness::LittleEndian);
    buffer.unwrap()
  }

  pub fn serialize_msg(self) -> Vec<u8> {
    match self.submessage {
      EntitySubmessage::AckNack(msg, _) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      EntitySubmessage::Data(_, _) => vec![],
      EntitySubmessage::DataFrag(_, _) => vec![],
      EntitySubmessage::Gap(msg) => msg.write_to_vec_with_ctx(
        Endianness::LittleEndian).unwrap(),
      EntitySubmessage::Heartbeat(msg, _) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      EntitySubmessage::HeartbeatFrag(msg) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      EntitySubmessage::NackFrag(msg) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
    }
  }
}