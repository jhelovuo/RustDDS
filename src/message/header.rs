use common::protocol_id;
use common::protocol_version;
use common::vendor_id;
use common::guid_prefix;
use common::locator;
use common::time;
use common::submessage_flag;
use message::validity_trait::Validity;

struct Header {
    protocol_id: protocol_id::ProtocolId_t,
    protocol_version: protocol_version::ProtocolVersion_t,
    vendor_id: vendor_id::VendorId_t,
    guid_prefix: guid_prefix::GuidPrefix_t,
}

impl Header {
    fn new(guid: guid_prefix::GuidPrefix_t) -> Header {
        Header {
            protocol_id: protocol_id::PROTOCOL_RTPS,
            protocol_version: protocol_version::PROTOCOLVERSION,
            vendor_id: vendor_id::VENDOR_UNKNOWN,
            guid_prefix: guid
        }
    }
}

impl Validity for Header {
    fn valid(&self) -> bool {
        !(self.protocol_id != protocol_id::PROTOCOL_RTPS ||
          self.protocol_version.major > protocol_version::PROTOCOLVERSION.major)
    }
}

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

struct Receiver {
    source_version: protocol_version::ProtocolVersion_t,
    source_vendor_id: vendor_id::VendorId_t,
    source_guid_prefix: guid_prefix::GuidPrefix_t,
    dest_guid_pregix: guid_prefix::GuidPrefix_t,
    unicast_reply_locator_list: locator::Locator_t,
    multicast_reply_locator_list: locator::Locator_t,
    have_timestamp: bool,
    timestapm: time::Time_t
}

impl Receiver {
    fn new(destination_guid_prefix: guid_prefix::GuidPrefix_t,
           unicast_reply_locator: locator::Locator_t,
           multicast_reply_locator: locator::Locator_t) -> Receiver {
        Receiver {
            source_version: protocol_version::PROTOCOLVERSION,
            source_vendor_id: vendor_id::VENDOR_UNKNOWN,
            source_guid_prefix: guid_prefix::GUIDPREFIX_UNKNOWN,
            dest_guid_pregix: destination_guid_prefix,
            unicast_reply_locator_list: locator::Locator_t {
                kind: unicast_reply_locator.kind,
                port: locator::LOCATOR_PORT_INVALID, // TODO: check if it is correct, page 35
                address: unicast_reply_locator.address
            },
            multicast_reply_locator_list: locator::Locator_t {
                kind: multicast_reply_locator.kind,
                port: locator::LOCATOR_PORT_INVALID, // TODO: check if it is correct, page 35
                address: multicast_reply_locator.address
            },
            have_timestamp: false,
            timestapm: time::TIME_INVALID
        }
    }
}
