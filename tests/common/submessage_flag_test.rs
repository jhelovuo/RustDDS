extern crate rtps;
use self::rtps::common::submessage_flag::{SubmessageFlag};

assert_ser_de!({
                   submessage_flag,
                   SubmessageFlag { flags: 0b10110100_u8 },
                   le = [0b10110100_u8],
                   be = [0b10110100_u8]
               });

#[test]
fn correct_bits_order() {
    let submessage_flag = SubmessageFlag { flags: 0b10110100_u8 };

    assert!(submessage_flag.flags & 0x01 == 0);
    assert!(submessage_flag.flags & (1 << 0) == 0);

    assert!(submessage_flag.flags & 0x80 != 0);
    assert!(submessage_flag.flags & (1 << 7) != 0);
}
