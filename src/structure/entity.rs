use crate::structure::guid::GUID;

/// Base class for all RTPS entities. RTPS Entity represents the class of
/// objects that are visible to other RTPS Entities on the network. As such,
/// RTPS Entity objects have a globally-unique identifier (GUID) and can be
/// referenced inside RTPS messages.
#[derive(Debug, PartialEq)]
pub struct EntityAttributes {
  /// Globally and uniquely identifies the RTPS Entity within the DDS domain.
  pub guid: GUID,
}

pub trait Entity {
  fn as_entity(&self) -> &EntityAttributes;
}
