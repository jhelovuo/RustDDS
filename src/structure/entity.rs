use crate::structure::guid::{GUID, EntityId, GuidPrefix};

/// Base class for all RTPS entities. RTPS Entity represents the class of
/// objects that are visible to other RTPS Entities on the network. As such,
/// RTPS Entity objects have a globally-unique identifier (GUID) and can be
/// referenced inside RTPS messages.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EntityAttributes {
  /// Globally and uniquely identifies the RTPS Entity within the DDS domain.
  pub guid: GUID,
}

impl EntityAttributes {
  pub fn new(guid: GUID) -> EntityAttributes {
    EntityAttributes { guid: guid }
  }

}

/// RTPS entity (for usage, DomainParticipant, DataReader and DataWriter implement this)
pub trait Entity {
  //fn as_entity(&self) -> &EntityAttributes;
  // This seems quite redundant

  fn get_guid(&self) -> GUID;

  fn get_entity_id(&self) -> EntityId {
    self.get_guid().entityId
  }
  fn get_guid_prefix(&self) -> GuidPrefix {
    self.get_guid().guidPrefix
  }
}
