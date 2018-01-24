#[derive(PartialOrd, PartialEq, Ord, Eq)]
pub struct ProtocolVersion_t {
    pub major: u8,
    pub minor: u8
}

pub const PROTOCOLVERSION_1_0: ProtocolVersion_t = ProtocolVersion_t { major: 1, minor: 0 };
pub const PROTOCOLVERSION_1_1: ProtocolVersion_t = ProtocolVersion_t { major: 1, minor: 1 };
pub const PROTOCOLVERSION_2_0: ProtocolVersion_t = ProtocolVersion_t { major: 2, minor: 0 };
pub const PROTOCOLVERSION_2_1: ProtocolVersion_t = ProtocolVersion_t { major: 2, minor: 1 };
pub const PROTOCOLVERSION_2_2: ProtocolVersion_t = ProtocolVersion_t { major: 2, minor: 2 };

pub const PROTOCOLVERSION: ProtocolVersion_t = PROTOCOLVERSION_2_2;
