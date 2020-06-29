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
use crate::messages::submessages::submessage_flag::SubmessageFlag;

#[derive(Debug, PartialEq)]
pub enum EntitySubmessage {
  AckNack(AckNack, SubmessageFlag),
  Data(Data, SubmessageFlag),
  DataFrag(DataFrag, SubmessageFlag),
  Gap(Gap),
  Heartbeat(Heartbeat, SubmessageFlag),
  HeartbeatFrag(HeartbeatFrag),
  NackFrag(NackFrag),
}

#[derive(Debug, PartialEq)]
pub enum InterpreterSubmessage {
  InfoSource(InfoSource),
  InfoDestination(InfoDestination),
  InfoReply(InfoReply, SubmessageFlag),
  InfoTimestamp(InfoTimestamp, SubmessageFlag),
  //Pad(Pad),
}
