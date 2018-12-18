use crate::common::guid_prefix::GuidPrefix_t;
use crate::common::protocol_version::ProtocolVersion_t;
use crate::common::vendor_id::VendorId_t;

/// This message modifies the logical source of the Submessages
/// that follow.
struct InfoSource {
    pub protocol_version: ProtocolVersion_t,
    pub vendor_id: VendorId_t,
    pub guid_prefix: GuidPrefix_t,
}
