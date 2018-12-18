use crate::common::locator::LocatorList_t;
use crate::message::validity_trait::Validity;

/// This message is sent from an RTPS Reader to an RTPS Writer.
/// It contains explicit information on where to send a reply
/// to the Submessages that follow it within the same message.
struct InfoReply {
    pub unicast_locator_list: LocatorList_t,
    pub multicast_locator_list: LocatorList_t,
}
