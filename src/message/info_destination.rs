use common::submessage_flag::{SubmessageFlag};
use common::guid_prefix::{GuidPrefix_t};
use message::submessage_header::{SubmessageHeader};
use message::validity_trait::Validity;

/// This message is sent from an RTPS Writer to an RTPS Reader
/// to modify the GuidPrefix used to interpret the Reader entityIds
/// appearing in the Submessages that follow it.
struct InfoDestination {
    submessage_header: SubmessageHeader,
    pub guid_prefix: GuidPrefix_t
}

impl InfoDestination {
    /// Indicates endianness. Returns true if big-endian, false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x01 != 0
    }
}

impl Validity for InfoDestination {
    fn valid(&self) -> bool {
        true
    }
}
