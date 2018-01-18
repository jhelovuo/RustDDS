use common::GuidPrefix;
use common::EntityId;

#[derive(PartialOrd, PartialEq, Ord, Eq)]
pub struct GUID_t {
    pub guidPrefix: GuidPrefix::GuidPrefix_t,
    pub entityId: EntityId::EntityId_t
}

pub const GUID_UNKNOWN: GUID_t = GUID_t { guidPrefix: GuidPrefix::GUIDPREFIX_UNKNOWN, entityId: EntityId::ENTITY_UNKNOWN };
