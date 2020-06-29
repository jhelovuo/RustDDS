use crate::messages::submessages::submessage_header::SubmessageHeader;
use crate::messages::submessages::submessage_kind::SubmessageKind;
use crate::messages::submessages::submessage::EntitySubmessage;
use speedy::{Readable, Writable, Endianness};

use crate::messages::submessages::ack_nack::*;
use crate::messages::submessages::gap::*;
use crate::messages::submessages::heartbeat::*;
use crate::messages::submessages::heartbeat_frag::*;
use crate::messages::submessages::nack_frag::*;


#[derive(Debug, PartialEq)]
pub struct SubMessage {
  pub header: SubmessageHeader,
  pub submessage: EntitySubmessage,
}

impl<'a> SubMessage
where
// T: Readable<'a, Endianness> + Writable<Endianness>,
{
  pub fn deserialize_header(context: Endianness, buffer: &'a [u8]) -> SubmessageHeader {
    SubmessageHeader::read_from_buffer_with_ctx(context, buffer).unwrap()
  }

  pub fn deserialize_msg(
    msgheader: &SubmessageHeader,
    context: Endianness,
    buffer: &'a [u8],
  ) -> Option<EntitySubmessage> {
    match msgheader.submessage_id {
      SubmessageKind::PAD => None, //Todo
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
      SubmessageKind::INFO_TS => None, //Todo
      SubmessageKind::INFO_SRC => None, //Todo
      SubmessageKind::INFO_REPLAY_IP4 => None, //Todo
      SubmessageKind::INFO_DST => None, //Todo
      SubmessageKind::INFO_REPLAY => None, //Todo
      SubmessageKind::NACK_FRAG => Some(EntitySubmessage::NackFrag(
        NackFrag::read_from_buffer_with_ctx(context, buffer).unwrap(),
      )),
      SubmessageKind::HEARTBEAT_FRAG => Some(EntitySubmessage::HeartbeatFrag(
        HeartbeatFrag::read_from_buffer_with_ctx(context, buffer).unwrap(),
      )),
      SubmessageKind::DATA => None,
      SubmessageKind::DATA_FRAG => None,
      _ => None, // Tarkista tää?
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
