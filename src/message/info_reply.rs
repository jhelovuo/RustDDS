use crate::common::locator::LocatorList_t;
use crate::common::submessage_flag::SubmessageFlag;
use crate::message::submessage_header::SubmessageHeader;
use crate::message::validity_trait::Validity;

/// This message is sent from an RTPS Reader to an RTPS Writer.
/// It contains explicit information on where to send a reply
/// to the Submessages that follow it within the same message.
struct InfoReply {
    submessage_header: SubmessageHeader,
    pub unicast_locator_list: LocatorList_t,
    pub multicast_locator_list: LocatorList_t,
}

impl InfoReply {
    /// Indicates endianness. Returns true if big-endian, false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x01 != 0
    }

    /// Indicates whether the Submessage also contains a multicast address
    pub fn multicast_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x02 != 0
    }
}

impl Validity for InfoReply {
    fn valid(&self) -> bool {
        true
    }
}
