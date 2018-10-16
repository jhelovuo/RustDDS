use crate::common::protocol_version;
use crate::common::vendor_id;
use crate::common::guid_prefix;
use crate::common::locator;
use crate::common::time;

pub struct MessageReceiver {
    pub source_version: protocol_version::ProtocolVersion_t,
    pub source_vendor_id: vendor_id::VendorId_t,
    pub source_guid_prefix: guid_prefix::GuidPrefix_t,
    pub dest_guid_prefix: guid_prefix::GuidPrefix_t,
    pub unicast_reply_locator_list: locator::LocatorList_t,
    pub multicast_reply_locator_list: locator::LocatorList_t,
    pub have_timestamp: bool,
    pub timestamp: time::Time_t
}

impl MessageReceiver {
    pub fn new(locator_kind: locator::LocatorKind_t) -> MessageReceiver {
        MessageReceiver {
            source_version: protocol_version::PROTOCOLVERSION,
            source_vendor_id: vendor_id::VENDOR_UNKNOWN,
            source_guid_prefix: guid_prefix::GUIDPREFIX_UNKNOWN,
            dest_guid_prefix: guid_prefix::GUIDPREFIX_UNKNOWN,
            unicast_reply_locator_list: vec![locator::Locator_t {
                kind: locator_kind,
                address: locator::LOCATOR_ADDRESS_INVALID,
                port: locator::LOCATOR_PORT_INVALID
            }],
            multicast_reply_locator_list: vec![locator::Locator_t {
                kind: locator_kind,
                address: locator::LOCATOR_ADDRESS_INVALID,
                port: locator::LOCATOR_PORT_INVALID
            }],
            have_timestamp: false,
            timestamp: time::TIME_INVALID
        }
    }
}
