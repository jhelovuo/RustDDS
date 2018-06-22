extern crate rtps;
extern crate serde;
extern crate cdr;
extern crate time;

use self::cdr::{CdrLe, CdrBe, Infinite, serialize, deserialize};
use self::rtps::common::time::{Time_t, TIME_INFINITE};
use common::tests::{remove_cdr_header};
use self::time::at_utc;

// #[test]
// fn serialize_deserialize() {
    
//     let encoded_le = serialize::<_, _, CdrLe>(&time, Infinite).unwrap();
//     let encoded_be = serialize::<_, _, CdrBe>(&time, Infinite).unwrap();

//     /// verify sample from wireshark
//     assert_eq!(vec![0x78, 0x6E, 0x8C, 0x5A, 0x7E, 0xE0, 0x2A, 0x4F],
//                remove_cdr_header(&encoded_be));

//     let decoded_le = deserialize::<Time_t>(&encoded_le[..]).unwrap();
//     let decoded_be = deserialize::<Time_t>(&encoded_be[..]).unwrap();

//     assert!(time == decoded_le);
//     assert!(time == decoded_be);
// }

#[test]
fn wireshark_payload() {
    let raw_data = vec![0x00, 0x01, 0x00, 0x00, 0x78, 0x6E, 0x8C, 0x5A, 0x7E, 0xE0, 0x2A, 0x4F];
    let decoded = deserialize::<Time_t>(&raw_data[..]).unwrap();

    let dupa: time::Timespec = decoded.into();
    println!("{:?}", time::at_utc(dupa));
}
