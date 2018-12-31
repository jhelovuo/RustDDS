use crate::common::guid_prefix::GuidPrefix_t;
use crate::common::locator::{LocatorKind_t, LocatorList_t, Locator_t};
use crate::common::protocol_version::ProtocolVersion_t;
use crate::common::time::Time_t;
use crate::common::vendor_id::VendorId_t;
use crate::message::submessage::EntitySubmessage;

use bytes::BytesMut;
use tokio::codec::Decoder;

pub struct MessageReceiver {
    pub source_version: ProtocolVersion_t,
    pub source_vendor_id: VendorId_t,
    pub source_guid_prefix: GuidPrefix_t,
    pub dest_guid_prefix: GuidPrefix_t,
    pub unicast_reply_locator_list: LocatorList_t,
    pub multicast_reply_locator_list: LocatorList_t,
    pub have_timestamp: bool,
    pub timestamp: Time_t,
}

impl MessageReceiver {
    pub fn new(locator_kind: LocatorKind_t) -> MessageReceiver {
        MessageReceiver {
            source_version: ProtocolVersion_t::PROTOCOLVERSION,
            source_vendor_id: VendorId_t::VENDOR_UNKNOWN,
            source_guid_prefix: GuidPrefix_t::GUIDPREFIX_UNKNOWN,
            dest_guid_prefix: GuidPrefix_t::GUIDPREFIX_UNKNOWN,
            unicast_reply_locator_list: vec![Locator_t {
                kind: locator_kind.clone(),
                address: Locator_t::LOCATOR_ADDRESS_INVALID,
                port: Locator_t::LOCATOR_PORT_INVALID,
            }],
            multicast_reply_locator_list: vec![Locator_t {
                kind: locator_kind,
                address: Locator_t::LOCATOR_ADDRESS_INVALID,
                port: Locator_t::LOCATOR_PORT_INVALID,
            }],
            have_timestamp: false,
            timestamp: Time_t::TIME_INVALID,
        }
    }
}

impl Decoder for MessageReceiver {
    type Item = EntitySubmessage;
    type Error = std::io::Error;

    fn decode(&mut self, _src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

}
