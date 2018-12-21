use crate::common::guid_prefix;
use crate::common::locator;
use crate::common::protocol_id;
use crate::common::protocol_version;
use crate::common::time;
use crate::common::vendor_id;
use crate::message::submessage_flag;
use crate::message::validity_trait::Validity;

#[derive(Debug, Readable, Writable, PartialEq)]
pub struct Header {
    pub protocol_id: protocol_id::ProtocolId_t,
    pub protocol_version: protocol_version::ProtocolVersion_t,
    pub vendor_id: vendor_id::VendorId_t,
    pub guid_prefix: guid_prefix::GuidPrefix_t,
}

impl Header {
    pub fn new(guid: guid_prefix::GuidPrefix_t) -> Header {
        Header {
            protocol_id: protocol_id::PROTOCOL_RTPS,
            protocol_version: protocol_version::PROTOCOLVERSION,
            vendor_id: vendor_id::VENDOR_UNKNOWN,
            guid_prefix: guid,
        }
    }
}

impl Validity for Header {
    fn valid(&self) -> bool {
        !(self.protocol_id != protocol_id::PROTOCOL_RTPS
            || self.protocol_version.major > protocol_version::PROTOCOLVERSION.major)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_protocol_version_major() {
        let mut header = Header::new(guid_prefix::GUIDPREFIX_UNKNOWN);

        header.protocol_version = protocol_version::PROTOCOLVERSION_1_0;
        assert!(header.valid());

        header.protocol_version = protocol_version::PROTOCOLVERSION;
        assert!(header.valid());

        header.protocol_version.major += 1;
        assert!(!header.valid());
    }

    #[test]
    fn header_protocol_id_same_as_rtps() {
        let mut header = Header::new(guid_prefix::GUIDPREFIX_UNKNOWN);

        header.protocol_id = protocol_id::PROTOCOL_RTPS;
        assert!(header.valid());
    }

    serialization_test!( type = Header,
    {
        header_with_unknown_guid_prefix,
        Header::new(guid_prefix::GUIDPREFIX_UNKNOWN),
        le = [0x52, 0x54, 0x50, 0x53, // protocol_id
              0x02, 0x02,             // protocol_verison
              0x00, 0x00,             // vendor_id
              0x00, 0x00, 0x00, 0x00, // guid_prefix
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00],
        be = [0x52, 0x54, 0x50, 0x53,
              0x02, 0x02,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00]
    });
}
