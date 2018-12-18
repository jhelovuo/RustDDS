use crate::message::acknack::AckNack;
use crate::message::data::Data;
use crate::message::gap::Gap;
use crate::message::heartbeat::HeartBeat;

#[derive(Debug)]
pub enum Submessage {
    AckNack(AckNack),
    Data(Data),
    Gap(Gap),
    HeartBeat(HeartBeat),
}
