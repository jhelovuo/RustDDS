use crate::structure::guid::{GUID, EntityId, GuidPrefix};

/// Base class for all RTPS entities. RTPS Entity represents the class of
/// objects that are visible to other RTPS Entities on the network. As such,
/// RTPS Entity objects have a globally-unique identifier (GUID) and can be
/// referenced inside RTPS messages.
#[derive(Debug, PartialEq)]
pub struct EntityAttributes {
  /// Globally and uniquely identifies the RTPS Entity within the DDS domain.
  pub guid: GUID,
}

impl EntityAttributes {
  pub fn new(guid: GUID) -> EntityAttributes {
    EntityAttributes { guid: guid }
  }

  pub fn as_usize(&self) -> usize {
    self.guid.entityId.as_usize()
  }
}

pub trait Entity {
  fn as_entity(&self) -> &EntityAttributes;

  fn get_guid(&self) -> GUID {
    self.as_entity().guid
  }
  fn get_entity_id(&self) -> EntityId {
    self.as_entity().guid.entityId
  }
  fn get_guid_prefix(&self) -> GuidPrefix {
    self.as_entity().guid.guidPrefix
  }
}
