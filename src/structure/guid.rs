use crate::structure::entity_id::EntityId_t;
use crate::structure::guid_prefix::GuidPrefix_t;
use speedy::{Readable, Writable};

#[derive(Copy, Clone, Debug, Default, PartialOrd, PartialEq, Ord, Eq, Readable, Writable)]
pub struct GUID_t {
  pub guidPrefix: GuidPrefix_t,
  pub entityId: EntityId_t,
}

impl GUID_t {
  pub const GUID_UNKNOWN: GUID_t = GUID_t {
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
      GUID_t {
        entityId: EntityId_t::ENTITYID_UNKNOWN,
        guidPrefix: GuidPrefix_t::GUIDPREFIX_UNKNOWN
      },
      GUID_t::GUID_UNKNOWN
    );
  }

  serialization_test!( type = GUID_t,
      {
          guid_unknown,
          GUID_t::GUID_UNKNOWN,
          le = [0x00; 16],
          be = [0x00; 16]
      },
      {
          guid_default,
          GUID_t::default(),
          le = [0x00; 16],
          be = [0x00; 16]
      },
      {
          guid_entity_id_on_the_last_position,
          GUID_t {
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
