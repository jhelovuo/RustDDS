use crate::messages::protocol_version::ProtocolVersion_t;
use crate::messages::vendor_id::VendorId_t;
use crate::structure::guid_prefix::GuidPrefix_t;

/// This message modifies the logical source of the Submessages
/// that follow.
#[derive(Debug, PartialEq)]
pub struct InfoSource {
    pub protocol_version: ProtocolVersion_t,
    pub vendor_id: VendorId_t,
    pub guid_prefix: GuidPrefix_t,
}
