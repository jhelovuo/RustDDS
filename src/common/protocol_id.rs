use message::validity_trait::Validity;

#[derive(PartialOrd, PartialEq, Ord, Eq)]
pub struct ProtocolId_t {
    protocol_id: [char;4]
}

pub const PROTOCOL_RTPS: ProtocolId_t = ProtocolId_t { protocol_id: ['R','T','P','S'] };

impl Validity for ProtocolId_t {
    fn valid(&self) -> bool {
        self.protocol_id == PROTOCOL_RTPS.protocol_id
    }
}
