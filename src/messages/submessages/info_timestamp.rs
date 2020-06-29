use crate::structure::time::Timestamp;
use speedy::{Readable, Writable};

/// This message modifies the logical source of the Submessages
/// that follow.
#[derive(Debug, PartialEq, Readable, Writable)]
pub struct InfoTimestamp {
  /// Contains the timestamp that should be used to interpret the
  /// subsequent Submessages
  ///
  /// Present only if the InvalidateFlag is not set in the header.
  pub timestamp: Timestamp,
}
