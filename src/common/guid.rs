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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_unknown_is_a_combination_of_unknown_members() {
        assert_eq!(Guid_t {
            entityId: entity_id::ENTITY_UNKNOWN,
            guidPrefix: guid_prefix::GUIDPREFIX_UNKNOWN
        }, GUID_UNKNOWN);
    }

    assert_ser_de!(
        {
            guid_unknown,
            GUID_UNKNOWN,
            le = [0x00; 16],
            be = [0x00; 16]
        },
        {
            guid_default,
            Guid_t::default(),
            le = [0x00; 16],
            be = [0x00; 16]
        },
        {
            guid_entity_id_on_the_last_position,
            Guid_t {
                entityId: entity_id::ENTITY_PARTICIPANT,
                ..Default::default()
            },
            le = [0x00, 0x00, 0x00, 0x00,
                  0x00, 0x00, 0x00, 0x00,
                  0x00, 0x00, 0x00, 0x00,
                  0x00, 0x00, 0x01, 0xC1],
            be = [0x00, 0x00, 0x00, 0x00,
                  0x00, 0x00, 0x00, 0x00,
                  0x00, 0x00, 0x00, 0x00,
                  0x00, 0x00, 0x01, 0xC1]
        }
    );
}
