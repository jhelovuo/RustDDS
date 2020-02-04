use crate::messages::ack_nack::AckNack;
use crate::messages::data::Data;
use crate::messages::data_frag::DataFrag;
use crate::messages::gap::Gap;
use crate::messages::heartbeat::Heartbeat;
use crate::messages::heartbeat_frag::HeartbeatFrag;
use crate::messages::info_destination::InfoDestination;
use crate::messages::info_reply::InfoReply;
use crate::messages::info_source::InfoSource;
use crate::messages::info_timestamp::InfoTimestamp;
use crate::messages::nack_frag::NackFrag;
use crate::messages::submessage_flag::SubmessageFlag;

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
    // Pad(Pad),
}
