#[derive(PartialOrd, PartialEq, Ord, Eq)]
pub struct EntityId_t {
    pub entityKey: [u8; 3],
    pub entityKind: u8
}

pub const ENTITY_UNKNOWN: EntityId_t = EntityId_t { entityKey: [0x00; 3], entityKind: 0x00 };
pub const ENTITY_PARTICIPANT: EntityId_t = EntityId_t { entityKey: [0x00, 0x00, 0x01], entityKind: 0xC1 };
pub const ENTITY_SEDP_BUILTIN_TOPIC_WRITER: EntityId_t = EntityId_t { entityKey: [0x00, 0x00, 0x02], entityKind: 0xC2 };
pub const ENTITY_SEDP_BUILTIN_TOPIC_READER: EntityId_t = EntityId_t { entityKey: [0x00, 0x00, 0x02], entityKind: 0xC7 };
pub const ENTITY_SEDP_BUILTIN_PUBLICATIONS_WRITER: EntityId_t = EntityId_t { entityKey: [0x00, 0x00, 0x03], entityKind: 0xC2 };
pub const ENTITY_SEDP_BUILTIN_PUBLICATIONS_READER: EntityId_t = EntityId_t { entityKey: [0x00, 0x00, 0x03], entityKind: 0xC7 };
pub const ENTITY_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER: EntityId_t = EntityId_t { entityKey: [0x00, 0x00, 0x04], entityKind: 0xC2 };
pub const ENTITY_SEDP_BUILTIN_SUBSCRIPTIONS_READER: EntityId_t = EntityId_t { entityKey: [0x00, 0x00, 0x04], entityKind: 0xC7 };
pub const ENTITY_SPDP_BUILTIN_PARTICIPANT_WRITER: EntityId_t = EntityId_t { entityKey: [0x00, 0x01, 0x00], entityKind: 0xC2 };
pub const ENTITY_SPDP_BUILTIN_PARTICIPANT_READER: EntityId_t = EntityId_t { entityKey: [0x00, 0x01, 0x00], entityKind: 0xC7 };
pub const ENTITY_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER: EntityId_t = EntityId_t { entityKey: [0x00, 0x02, 0x00], entityKind: 0xC2 };
pub const ENTITY_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER: EntityId_t = EntityId_t { entityKey: [0x00, 0x02, 0x00], entityKind: 0xC7 };
