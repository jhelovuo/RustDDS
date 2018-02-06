extern crate rtps;
extern crate serde;
extern crate cdr;

use self::cdr::{CdrLe, CdrBe, Infinite, serialize, deserialize};
use self::rtps::common::submessage_flag::{SubmessageFlag};
use common::tests::{remove_cdr_header};

#[test]
fn serialize_deserialize() {
    let submessage_flag = SubmessageFlag { flags: 0b10110100_u8 };

    let encoded_le = serialize::<_, _, CdrLe>(&submessage_flag, Infinite).unwrap();
    let encoded_be = serialize::<_, _, CdrBe>(&submessage_flag, Infinite).unwrap();

    /// serialization should not be endianness sensitive
    assert_eq!(remove_cdr_header(&encoded_be), remove_cdr_header(&encoded_le));

    /// verify order of bits
    assert_eq!(0b10110100_u8, remove_cdr_header(&encoded_be)[0]);

    /// should serialize to single u8 value
    assert_eq!(1, remove_cdr_header(&encoded_le).len());
    assert_eq!(1, remove_cdr_header(&encoded_be).len());

    let decoded_le = deserialize::<SubmessageFlag>(&encoded_le[..]).unwrap();
    let decoded_be = deserialize::<SubmessageFlag>(&encoded_be[..]).unwrap();

    assert!(submessage_flag == decoded_le);
    assert!(submessage_flag == decoded_be);
}

#[test]
fn correct_bits_order() {
    let submessage_flag = SubmessageFlag { flags: 0b10110100_u8 };

    assert!(submessage_flag.flags & 0x01 == 0);
    assert!(submessage_flag.flags & (1 << 0) == 0);

    assert!(submessage_flag.flags & 0x80 != 0);
    assert!(submessage_flag.flags & (1 << 7) != 0);
}
