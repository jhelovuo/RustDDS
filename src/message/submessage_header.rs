use crate::common::submessage_flag;
use crate::enum_number;

enum_number_u8!(SubmessageKind {
    PAD = 0x01,
    ACKNACK = 0x06,
    HEARTBEAT = 0x07,
    GAP = 0x08,
    INFO_TS = 0x09,
    INFO_SRC = 0x0c,
    INFO_REPLAY_IP4 = 0x0d,
    INFO_DST = 0x0e,
    INFO_REPLAY = 0x0f,
    NACK_FRAG = 0x12,
    HEARTBEAT_FRAG = 0x13,
    DATA = 0x15,
    DATA_FRAG = 0x16,
});

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmessageHeader {
    pub submessage_id: SubmessageKind,
    pub flags: submessage_flag::SubmessageFlag,
    pub submessage_length: u16
}
