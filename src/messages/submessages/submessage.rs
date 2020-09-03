use crate::messages::submessages::ack_nack::AckNack;
use crate::messages::submessages::data::Data;
use crate::messages::submessages::data_frag::DataFrag;
use crate::messages::submessages::gap::Gap;
use crate::messages::submessages::heartbeat::Heartbeat;
use crate::messages::submessages::heartbeat_frag::HeartbeatFrag;
use crate::messages::submessages::info_destination::InfoDestination;
use crate::messages::submessages::info_reply::InfoReply;
use crate::messages::submessages::info_source::InfoSource;
use crate::messages::submessages::info_timestamp::InfoTimestamp;
use crate::messages::submessages::nack_frag::NackFrag;
use crate::messages::submessages::submessage_flag::*;

use speedy::{Writable, Writer, Context};
use enumflags2::BitFlags;

//TODO: These messages are structured a bit oddly. Why is flags separate from the submessage proper?

#[derive(Debug, PartialEq)]
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
// 1) we cannot implement Writable for *Flags defined using enumflags2, as they are foreign types (coherence rules)
// 2) Writer should not use any enum variant tag in this type, as we have SubmessageHeader already.
impl<C:Context> Writable<C> for EntitySubmessage {
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    match self {
      EntitySubmessage::AckNack(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }
      EntitySubmessage::Data(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }
      EntitySubmessage::DataFrag(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }
      EntitySubmessage::Gap(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }
      EntitySubmessage::Heartbeat(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }
      EntitySubmessage::HeartbeatFrag(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }
      EntitySubmessage::NackFrag(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }        
    }
  }
}


#[derive(Debug, PartialEq)]
pub enum InterpreterSubmessage {
  InfoSource(InfoSource, BitFlags<INFOSOURCE_Flags>),
  InfoDestination(InfoDestination, BitFlags<INFODESTINATION_Flags>),
  InfoReply(InfoReply, BitFlags<INFOREPLY_Flags>),
  InfoTimestamp(InfoTimestamp, BitFlags<INFOTIMESTAMP_Flags>),
  //Pad(Pad), // Pad message does not need to be processed above serialization layer
}

// See notes on impl Writer for EntitySubmessage
impl<C:Context> Writable<C> for InterpreterSubmessage {
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    match self {
      InterpreterSubmessage::InfoSource(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }
      InterpreterSubmessage::InfoDestination(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }
      InterpreterSubmessage::InfoReply(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }
      InterpreterSubmessage::InfoTimestamp(s,f) => { writer.write_value(s)?; writer.write_u8( f.bits() ) }
    }
  }
}
