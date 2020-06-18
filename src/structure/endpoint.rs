use crate::structure::locator;
use crate::structure::reliability_kind;
use crate::structure::topic_kind;

/// Specialization of RTPS Entity representing the objects that can be
/// communication endpoints. That is, the objects that can be the sources or
/// destinations of RTPS messages.
pub struct EndpointAttributes {
  /// Used to indicate whether the Endpoint is associated with a DataType that
  /// has defined some fields as containing the DDS key.
  pub topic_kind: topic_kind::TopicKind,

  /// The levels of reliability supported by the Endpoint.
  pub reliability_level: reliability_kind::ReliabilityKind,

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
