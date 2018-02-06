extern crate rtps;
extern crate serde;
extern crate cdr;

use self::cdr::{CdrLe, CdrBe, Infinite, serialize, deserialize};
use self::rtps::common::count::{Count_t};
use common::tests::{remove_cdr_header};

#[test]
fn serialize_deserialize() {
    let count = Count_t { value: 0x1234ABCD_i32 };

    let encoded_le = serialize::<_, _, CdrLe>(&count, Infinite).unwrap();
    let encoded_be = serialize::<_, _, CdrBe>(&count, Infinite).unwrap();

    /// should serialize into 4 bytes
    assert_eq!(4, remove_cdr_header(&encoded_le).len());
    assert_eq!(4, remove_cdr_header(&encoded_be).len());

    let decoded_le = deserialize::<Count_t>(&encoded_le[..]).unwrap();
    let decoded_be = deserialize::<Count_t>(&encoded_be[..]).unwrap();

    assert!(count == decoded_le);
    assert!(count == decoded_be);
}
