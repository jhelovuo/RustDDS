use crate::messages::submessages::submessage_header::SubmessageHeader;
use crate::messages::submessages::submessage::EntitySubmessage;
use speedy::{Readable, Writable, Endianness};

use crate::messages::submessages::ack_nack::*;
use crate::messages::submessages::gap::*;
use crate::messages::submessages::heartbeat::*;
use crate::messages::submessages::heartbeat_frag::*;
use crate::messages::submessages::nack_frag::*;

#[derive(Debug, PartialEq)]
pub struct SubMessage {
  header: SubmessageHeader,
  submessage: EntitySubmessage,
}

impl<'a> SubMessage
where
// T: Readable<'a, Endianness> + Writable<Endianness>,
{
  pub fn deserialize_header(context: Endianness, buffer: &'a [u8]) -> SubmessageHeader {
    SubmessageHeader::read_from_buffer_with_ctx(context, buffer).unwrap()
  }

  pub fn deserialize_msg(
    msgtype: EntitySubmessage,
    context: Endianness,
    buffer: &'a [u8],
  ) -> Option<EntitySubmessage> {
    match msgtype {
      EntitySubmessage::AckNack(_, flag) => Some(EntitySubmessage::AckNack(
        AckNack::read_from_buffer_with_ctx(context, buffer).unwrap(),
        flag,
      )),
      EntitySubmessage::Data(_, _) => None,
      EntitySubmessage::DataFrag(_, _) => None,
      EntitySubmessage::Gap(_) => Some(EntitySubmessage::Gap(
        Gap::read_from_buffer_with_ctx(context, buffer).unwrap(),
      )),
      EntitySubmessage::Heartbeat(_, flag) => Some(EntitySubmessage::Heartbeat(
        Heartbeat::read_from_buffer_with_ctx(context, buffer).unwrap(),
        flag,
      )),
      EntitySubmessage::HeartbeatFrag(_) => Some(EntitySubmessage::HeartbeatFrag(
        HeartbeatFrag::read_from_buffer_with_ctx(context, buffer).unwrap(),
      )),
      EntitySubmessage::NackFrag(_) => Some(EntitySubmessage::NackFrag(
        NackFrag::read_from_buffer_with_ctx(context, buffer).unwrap(),
      )),
    }
  }

  pub fn serialize_header(self) -> Vec<u8> {
    let buffer = self.header.write_to_vec_with_ctx(Endianness::LittleEndian);
    buffer.unwrap()
  }

  pub fn serialize_msg(self) -> Vec<u8> {
    match self.submessage {
      EntitySubmessage::AckNack(msg, _) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      EntitySubmessage::Data(_, _) => vec![],
      EntitySubmessage::DataFrag(_, _) => vec![],
      EntitySubmessage::Gap(msg) => msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap(),
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
