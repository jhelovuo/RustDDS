extern crate rtps;
extern crate time;

use self::rtps::common::vendor_id::{VENDOR_UNKNOWN};

assert_ser_de!({
    vendor_unknown,
    VENDOR_UNKNOWN,
    le = [0x00, 0x00],
    be = [0x00, 0x00]
});
