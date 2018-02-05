extern crate rtps;
extern crate serde;
extern crate cdr;

use self::cdr::{CdrLe, CdrBe, Infinite, serialize, deserialize};
use self::rtps::common::protocol_id::{ProtocolId_t, PROTOCOL_RTPS};
use common::protocol_id_test::rtps::message::validity_trait::Validity;

#[test]
fn serialize_deserialize() {
    let protocol_id = PROTOCOL_RTPS;

    let encoded_le = serialize::<_, _, CdrLe>(&protocol_id, Infinite).unwrap();
    let encoded_be = serialize::<_, _, CdrBe>(&protocol_id, Infinite).unwrap();

    /// serialization should not be endianness sensitive
    // assert_eq!(encoded_be, encoded_le); // TODO: add serialize without cdr header

    /// verify sample from wireshark
    assert_eq!(vec![0x52, 0x54, 0x50, 0x53], encoded_be);

    let decoded_le = deserialize::<ProtocolId_t>(&encoded_le[..]).unwrap();
    let decoded_be = deserialize::<ProtocolId_t>(&encoded_be[..]).unwrap();

    assert!(protocol_id == decoded_le);
    assert!(protocol_id == decoded_be);
}

#[test]
fn validity() {
    let protocol_id = PROTOCOL_RTPS;
    assert!(protocol_id.valid());
    let protocol_id = ProtocolId_t { protocol_id: ['S','P','T','R'] };
    assert!(!protocol_id.valid());
}
