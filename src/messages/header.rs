use crate::common::validity_trait::Validity;
use crate::messages::protocol_id::ProtocolId_t;
use crate::messages::protocol_version::ProtocolVersion_t;
use crate::messages::vendor_id::VendorId_t;
use crate::structure::guid_prefix::GuidPrefix_t;
use speedy_derive::{Readable, Writable};

#[derive(Debug, Readable, Writable, PartialEq)]
pub struct Header {
    pub protocol_id: ProtocolId_t,
    pub protocol_version: ProtocolVersion_t,
    pub vendor_id: VendorId_t,
    pub guid_prefix: GuidPrefix_t,
}

impl Header {
    pub fn new(guid: GuidPrefix_t) -> Header {
        Header {
            protocol_id: ProtocolId_t::PROTOCOL_RTPS,
            protocol_version: ProtocolVersion_t::PROTOCOLVERSION,
            vendor_id: VendorId_t::VENDOR_UNKNOWN,
            guid_prefix: guid,
        }
    }
}

impl Validity for Header {
    fn valid(&self) -> bool {
        !(self.protocol_id != ProtocolId_t::PROTOCOL_RTPS
            || self.protocol_version.major > ProtocolVersion_t::PROTOCOLVERSION.major)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_protocol_version_major() {
        let mut header = Header::new(GuidPrefix_t::GUIDPREFIX_UNKNOWN);

        header.protocol_version = ProtocolVersion_t::PROTOCOLVERSION_1_0;
        assert!(header.valid());

        header.protocol_version = ProtocolVersion_t::PROTOCOLVERSION;
        assert!(header.valid());

        header.protocol_version.major += 1;
        assert!(!header.valid());
    }

    #[test]
    fn header_protocol_id_same_as_rtps() {
        let mut header = Header::new(GuidPrefix_t::GUIDPREFIX_UNKNOWN);

        header.protocol_id = ProtocolId_t::PROTOCOL_RTPS;
        assert!(header.valid());
    }

    serialization_test!( type = Header,
    {
        header_with_unknown_guid_prefix,
        Header::new(GuidPrefix_t::GUIDPREFIX_UNKNOWN),
        le = [0x52, 0x54, 0x50, 0x53, // protocol_id
              0x02, 0x04,             // protocol_verison
              0x00, 0x00,             // vendor_id
              0x00, 0x00, 0x00, 0x00, // guid_prefix
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00],
        be = [0x52, 0x54, 0x50, 0x53,
              0x02, 0x04,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00]
    });
}
