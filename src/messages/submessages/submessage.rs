use speedy::{Context, Writable, Writer};
use enumflags2::BitFlags;

use crate::{
  dds::data_types::EntityId,
  messages::submessages::{
    ack_nack::AckNack, data::Data, data_frag::DataFrag, gap::Gap, heartbeat::Heartbeat,
    heartbeat_frag::HeartbeatFrag, info_destination::InfoDestination, info_reply::InfoReply,
    info_source::InfoSource, info_timestamp::InfoTimestamp, nack_frag::NackFrag,
    submessage_flag::*,
  },
};

//TODO: These messages are structured a bit oddly. Why is flags separate from
// the submessage proper?

#[derive(Debug, PartialEq, Clone)]
pub enum EntitySubmessage {
  AckNack(AckNack, BitFlags<ACKNACK_Flags>),
  Data(Data, BitFlags<DATA_Flags>),
  DataFrag(DataFrag, BitFlags<DATAFRAG_Flags>),
  Gap(Gap, BitFlags<GAP_Flags>),
  Heartbeat(Heartbeat, BitFlags<HEARTBEAT_Flags>),
  HeartbeatFrag(HeartbeatFrag, BitFlags<HEARTBEATFRAG_Flags>),
  NackFrag(NackFrag, BitFlags<NACKFRAG_Flags>),
}

// we must write this manually, because
// 1) we cannot implement Writable for *Flags defined using enumflags2, as they
// are foreign types (coherence rules) 2) Writer should not use any enum variant
// tag in this type, as we have SubmessageHeader already.
impl<C: Context> Writable<C> for EntitySubmessage {
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    match self {
      EntitySubmessage::AckNack(s, _f) => writer.write_value(s),
      EntitySubmessage::Data(s, _f) => writer.write_value(s),
      EntitySubmessage::DataFrag(s, _f) => writer.write_value(s),
      EntitySubmessage::Gap(s, _f) => writer.write_value(s),
      EntitySubmessage::Heartbeat(s, _f) => writer.write_value(s),
      EntitySubmessage::HeartbeatFrag(s, _f) => writer.write_value(s),
      EntitySubmessage::NackFrag(s, _f) => writer.write_value(s),
    }
  }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum InterpreterSubmessage {
  InfoSource(InfoSource, BitFlags<INFOSOURCE_Flags>),
  InfoDestination(InfoDestination, BitFlags<INFODESTINATION_Flags>),
  InfoReply(InfoReply, BitFlags<INFOREPLY_Flags>),
  InfoTimestamp(InfoTimestamp, BitFlags<INFOTIMESTAMP_Flags>),
  //Pad(Pad), // Pad message does not need to be processed above serialization layer
}

// See notes on impl Writer for EntitySubmessage
impl<C: Context> Writable<C> for InterpreterSubmessage {
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    match self {
      InterpreterSubmessage::InfoSource(s, _f) => writer.write_value(s),
      InterpreterSubmessage::InfoDestination(s, _f) => writer.write_value(s),
      InterpreterSubmessage::InfoReply(s, _f) => writer.write_value(s),
      InterpreterSubmessage::InfoTimestamp(s, _f) => match s {
        InfoTimestamp { timestamp: None } => Ok(()), // serialization is empty string
        InfoTimestamp {
          timestamp: Some(ts),
        } => writer.write_value(ts),
      },
    }
  }
}

#[derive(Debug)]
pub enum AckSubmessage {
  AckNack(AckNack),
  NackFrag(NackFrag),
}

impl AckSubmessage {
  pub fn writer_id(&self) -> EntityId {
    match self {
      AckSubmessage::AckNack(a) => a.writer_id,
      AckSubmessage::NackFrag(a) => a.writer_id,
    }
  }
}
