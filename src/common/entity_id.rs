#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Ord, Eq)]
pub struct EntityId_t {
    pub entityKey: [u8; 3],
    pub entityKind: u8
}

impl Default for EntityId_t {
    fn default() -> EntityId_t {
        ENTITY_UNKNOWN
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    assert_ser_de!(
        {
            entity_unknown,
            ENTITY_UNKNOWN,
            le = [0x00, 0x00, 0x00, 0x00],
            be = [0x00, 0x00, 0x00, 0x00]
        },
        {
            entity_default,
            EntityId_t::default(),
            le = [0x00, 0x00, 0x00, 0x00],
            be = [0x00, 0x00, 0x00, 0x00]
        },
        {
            entity_participant,
            ENTITY_PARTICIPANT,
            le = [0x00, 0x00, 0x01, 0xC1],
            be = [0x00, 0x00, 0x01, 0xC1]
        },
        {
            entity_sedp_builtin_topic_writer,
            ENTITY_SEDP_BUILTIN_TOPIC_WRITER,
            le = [0x00, 0x00, 0x02, 0xC2],
            be = [0x00, 0x00, 0x02, 0xC2]
        },
        {
            entity_sedp_builtin_topic_reader,
            ENTITY_SEDP_BUILTIN_TOPIC_READER,
            le = [0x00, 0x00, 0x02, 0xC7],
            be = [0x00, 0x00, 0x02, 0xC7]
        },
        {
            entity_sedp_builtin_publications_writer,
            ENTITY_SEDP_BUILTIN_PUBLICATIONS_WRITER,
            le = [0x00, 0x00, 0x03, 0xC2],
            be = [0x00, 0x00, 0x03, 0xC2]
        },
        {
            entity_sedp_builtin_publications_reader,
            ENTITY_SEDP_BUILTIN_PUBLICATIONS_READER,
            le = [0x00, 0x00, 0x03, 0xC7],
            be = [0x00, 0x00, 0x03, 0xC7]
        },
        {
            entity_sedp_builtin_subscriptions_writer,
            ENTITY_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER,
            le = [0x00, 0x00, 0x04, 0xC2],
            be = [0x00, 0x00, 0x04, 0xC2]
        },
        {
            entity_sedp_builtin_subscriptions_reader,
            ENTITY_SEDP_BUILTIN_SUBSCRIPTIONS_READER,
            le = [0x00, 0x00, 0x04, 0xC7],
            be = [0x00, 0x00, 0x04, 0xC7]
        },
        {
            entity_spdp_builtin_participant_writer,
            ENTITY_SPDP_BUILTIN_PARTICIPANT_WRITER,
            le = [0x00, 0x01, 0x00, 0xC2],
            be = [0x00, 0x01, 0x00, 0xC2]
        },
        {
            entity_spdp_builtin_participant_reader,
            ENTITY_SPDP_BUILTIN_PARTICIPANT_READER,
            le = [0x00, 0x01, 0x00, 0xC7],
            be = [0x00, 0x01, 0x00, 0xC7]
        },
        {
            entity_p2p_builtin_participant_message_writer,
            ENTITY_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
            le = [0x00, 0x02, 0x00, 0xC2],
            be = [0x00, 0x02, 0x00, 0xC2]
        },
        {
            entity_p2p_builtin_participant_message_reader,
            ENTITY_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER,
            le = [0x00, 0x02, 0x00, 0xC7],
            be = [0x00, 0x02, 0x00, 0xC7]
        }
    );
}
