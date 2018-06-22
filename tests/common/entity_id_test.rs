extern crate rtps;
extern crate serde;
extern crate cdr;

use self::cdr::{CdrLe, CdrBe, Infinite, serialize, deserialize};
use self::rtps::common::entity_id::{EntityId_t, ENTITY_PARTICIPANT};
use common::tests::{remove_cdr_header};

#[test]
fn serialize_deserialize() {
    let entity_id = ENTITY_PARTICIPANT;

    let encoded_le = serialize::<_, _, CdrLe>(&entity_id, Infinite).unwrap();
    let encoded_be = serialize::<_, _, CdrBe>(&entity_id, Infinite).unwrap();

    /// verify sample from wireshark
    assert_eq!(vec![0x00, 0x00, 0x01, 0xC1], remove_cdr_header(&encoded_be));

    let decoded_le = deserialize::<EntityId_t>(&encoded_le[..]).unwrap();
    let decoded_be = deserialize::<EntityId_t>(&encoded_be[..]).unwrap();

    assert!(entity_id == decoded_le);
    assert!(entity_id == decoded_be);
}
