use crate::common::guid_prefix::GuidPrefix_t;

/// This message is sent from an RTPS Writer to an RTPS Reader
/// to modify the GuidPrefix used to interpret the Reader entityIds
/// appearing in the Submessages that follow it.
#[derive(Debug, PartialEq)]
pub struct InfoDestination {
    pub guid_prefix: GuidPrefix_t,
}
