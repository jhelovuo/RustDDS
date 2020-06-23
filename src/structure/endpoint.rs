use crate::structure::locator;
use crate::structure::topic_kind;

use speedy::{Readable, Writable};

#[derive(Debug, PartialEq, Eq, Readable, Writable)]
pub struct ReliabilityKind(u32);

impl ReliabilityKind {
  pub const BEST_EFFORT: ReliabilityKind = ReliabilityKind(1);
  pub const RELIABLE: ReliabilityKind = ReliabilityKind(2);
}

/// Specialization of RTPS Entity representing the objects that can be
/// communication endpoints. That is, the objects that can be the sources or
/// destinations of RTPS messages.
pub struct EndpointAttributes {
  /// Used to indicate whether the Endpoint is associated with a DataType that
  /// has defined some fields as containing the DDS key.
  pub topic_kind: topic_kind::TopicKind,

  /// The levels of reliability supported by the Endpoint.
  pub reliability_level: ReliabilityKind,

  /// List of unicast locators (transport, address, port combinations) that
  /// can be used to send messages to the Endpoint. The list may be empty.
  pub unicast_locator_list: locator::Locator,

  /// List of multicast locators (transport, address, port combinations) that
  /// can be used to send messages to the Endpoint. The list may be empty.
  pub multicast_locator_list: locator::Locator,
}

pub trait Endpoint {
  fn as_endpoint(&self) -> &EndpointAttributes;
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = ReliabilityKind,
  {
      reliability_kind_best_effort,
      ReliabilityKind::BEST_EFFORT,
      le = [0x01, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x01]
  },
  {
      reliability_kind_reliable,
      ReliabilityKind::RELIABLE,
      le = [0x02, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x02]
  });
}
