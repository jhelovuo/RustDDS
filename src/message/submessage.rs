use crate::message::acknack::AckNack;
use crate::message::data::Data;
use crate::message::gap::Gap;
use crate::message::heartbeat::Heartbeat;
use crate::message::info_source::InfoSource;
use crate::message::info_destination::InfoDestination;
use crate::message::info_reply::InfoReply;
use crate::message::info_timestamp::InfoTimestamp;

#[derive(Debug)]
pub enum EntitySubmessage {
    AckNack(AckNack),
    Data(Data),
    // DataFrag(DataFrag),
    Gap(Gap),
    Heartbeat(Heartbeat),
    // HeartbeatFrag(HeartbeatFrag),
    // NackFrag(NackFrag),
}

#[derive(Debug)]
pub enum InterpreterSubmessage {
    InfoSource(InfoSource),
    InfoDestination(InfoDestination),
    InfoReply(InfoReply),
    InfoTimestamp(InfoTimestamp),
    // Pad(Pad),
}
