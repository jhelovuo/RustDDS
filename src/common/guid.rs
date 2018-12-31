use crate::common::entity_id::EntityId_t;
use crate::common::guid_prefix::GuidPrefix_t;

#[derive(Debug, Default, PartialOrd, PartialEq, Ord, Eq, Readable, Writable)]
pub struct Guid_t {
    pub guidPrefix: GuidPrefix_t,
    pub entityId: EntityId_t,
}

impl Guid_t {
    pub const GUID_UNKNOWN: Guid_t = Guid_t {
        guidPrefix: GuidPrefix_t::GUIDPREFIX_UNKNOWN,
        entityId: EntityId_t::ENTITYID_UNKNOWN,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_unknown_is_a_combination_of_unknown_members() {
        assert_eq!(
            Guid_t {
                entityId: EntityId_t::ENTITYID_UNKNOWN,
                guidPrefix: GuidPrefix_t::GUIDPREFIX_UNKNOWN
            },
            Guid_t::GUID_UNKNOWN
        );
    }

    serialization_test!( type = Guid_t,
        {
            guid_unknown,
            Guid_t::GUID_UNKNOWN,
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
                entityId: EntityId_t::ENTITYID_PARTICIPANT,
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
