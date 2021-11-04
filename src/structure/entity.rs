use crate::structure::guid::{EntityId, GuidPrefix, GUID};

/// Base class for all RTPS entities. RTPS Entity represents the class of
/// objects that are visible to other RTPS Entities on the network. As such,
/// RTPS Entity objects have a globally-unique identifier (GUID) and can be
/// referenced inside RTPS messages.
/// (for usage, DomainParticipant, DataReader and DataWriter implement this)
/// RTPS 2.3 specification section 8.2.4
pub trait RTPSEntity {
  fn get_guid(&self) -> GUID;

  fn get_entity_id(&self) -> EntityId {
    self.get_guid().entity_id
  }
  fn get_guid_prefix(&self) -> GuidPrefix {
    self.get_guid().guid_prefix
  }
}
