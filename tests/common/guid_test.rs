extern crate rtps;

use self::rtps::common::guid::{Guid_t, GUID_UNKNOWN};
use self::rtps::common::guid_prefix::{GUIDPREFIX_UNKNOWN};
use self::rtps::common::entity_id::{ENTITY_UNKNOWN, ENTITY_PARTICIPANT};

assert_ser_de!({
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
                       entityId: ENTITY_PARTICIPANT,
                       ..Default::default()
                   },
                   le = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xC1],
                   be = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xC1]
               }
);

#[test]
fn guid_unknown_is_a_combination_of_unknown_members() {
    assert_eq!(Guid_t {
        entityId: ENTITY_UNKNOWN,
        guidPrefix: GUIDPREFIX_UNKNOWN
    }, GUID_UNKNOWN);
}
