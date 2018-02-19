use common::submessage_flag::{SubmessageFlag};
use common::time::{Timestamp};
use message::submessage_header::{SubmessageHeader};
use message::validity_trait::Validity;

/// This message modifies the logical source of the Submessages
/// that follow.
struct InfoTimestamp {
    submessage_header: SubmessageHeader,
    /// Contains the timestamp that should be used to interpret the 
    /// subsequent Submessages
    ///
    /// Present only if the InvalidateFlag is not set in the header.
    pub timestamp: Timestamp
}

impl InfoTimestamp {
    /// Indicates endianness. Returns true if big-endian, false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x01 != 0
    }

    /// Indicates whether subsequent Submessages should be considered 
    /// as having a timestamp or not.
    pub fn invalidate_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x02 != 0
    }
}

impl Validity for InfoTimestamp {
    fn valid(&self) -> bool {
        true
    }
}
