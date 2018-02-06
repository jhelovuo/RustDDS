use common::submessage_flag::{SubmessageFlag};
use common::protocol_version::{ProtocolVersion_t};
use common::vendor_id::{VendorId_t};
use common::guid_prefix::{GuidPrefix_t};
use message::submessage_header::{SubmessageHeader};
use message::validity_trait::Validity;

/// This message modifies the logical source of the Submessages
/// that follow.
struct InfoSource {
    submessage_header: SubmessageHeader,
    pub protocol_version: ProtocolVersion_t,
    pub vendor_id: VendorId_t,
    pub guid_prefix: GuidPrefix_t
}

impl InfoSource {
    /// Indicates endianness. Returns true if big-endian, false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x01 != 0
    }
}

impl Validity for InfoSource {
    fn valid(&self) -> bool {
        true
    }
}
