use crate::common::guid_prefix;
use crate::common::entity_id;

#[derive(Debug, Default, Serialize, Deserialize, PartialOrd, PartialEq, Ord, Eq)]
pub struct Guid_t {
    pub guidPrefix: guid_prefix::GuidPrefix_t,
    pub entityId: entity_id::EntityId_t
}

pub const GUID_UNKNOWN: Guid_t = Guid_t {
    guidPrefix: guid_prefix::GUIDPREFIX_UNKNOWN,
    entityId: entity_id::ENTITY_UNKNOWN
};
