use crate::common::guid_prefix::GuidPrefix_t;
use crate::message::validity_trait::Validity;

/// This message is sent from an RTPS Writer to an RTPS Reader
/// to modify the GuidPrefix used to interpret the Reader entityIds
/// appearing in the Submessages that follow it.
struct InfoDestination {
    pub guid_prefix: GuidPrefix_t,
}
