use crate::message::Validity;

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Ord, Eq)]
pub struct ProtocolId_t {
    pub protocol_id: [char;4]
}

pub const PROTOCOL_RTPS: ProtocolId_t = ProtocolId_t { protocol_id: ['R','T','P','S'] };

impl Validity for ProtocolId_t {
    fn valid(&self) -> bool {
        self.protocol_id == PROTOCOL_RTPS.protocol_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validity() {
        let protocol_id = PROTOCOL_RTPS;
        assert!(protocol_id.valid());
        let protocol_id = ProtocolId_t { protocol_id: ['S','P','T','R'] };
        assert!(!protocol_id.valid());
    }

    assert_ser_de!({
        protocol_rtps,
        PROTOCOL_RTPS,
        le = [0x52, 0x54, 0x50, 0x53],
        be = [0x52, 0x54, 0x50, 0x53]
    });
}
