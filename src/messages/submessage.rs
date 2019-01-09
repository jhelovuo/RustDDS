use crate::messages::ack_nack::AckNack;
use crate::messages::data::Data;
use crate::messages::gap::Gap;
use crate::messages::heartbeat::Heartbeat;
use crate::messages::heartbeat_frag::HeartbeatFrag;
use crate::messages::info_destination::InfoDestination;
use crate::messages::info_reply::InfoReply;
use crate::messages::info_source::InfoSource;
use crate::messages::info_timestamp::InfoTimestamp;
use crate::messages::nack_frag::NackFrag;

#[derive(Debug)]
pub enum EntitySubmessage {
    AckNack(AckNack),
    Data(Data),
    // DataFrag(DataFrag),
    Gap(Gap),
    Heartbeat(Heartbeat),
    HeartbeatFrag(HeartbeatFrag),
    NackFrag(NackFrag),
}

#[derive(Debug)]
pub enum InterpreterSubmessage {
    InfoSource(InfoSource),
    InfoDestination(InfoDestination),
    InfoReply(InfoReply),
    InfoTimestamp(InfoTimestamp),
    // Pad(Pad),
}
