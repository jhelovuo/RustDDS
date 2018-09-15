extern crate rtps;

use self::rtps::common::count::{Count_t};

assert_ser_de!({
                   count_test_one,
                   Count_t { value: 1 },
                   le = [0x01, 0x00, 0x00, 0x00],
                   be = [0x00, 0x00, 0x00, 0x01]
               },
               {
                   count_test_min,
                   Count_t { value: 0 },
                   le = [0x00, 0x00, 0x00, 0x00],
                   be = [0x00, 0x00, 0x00, 0x00]
               },
               {
                   count_test_high,
                   Count_t { value: 0x3BCDEF01 },
                   le = [0x01, 0xEF, 0xCD, 0x3B],
                   be = [0x3B, 0xCD, 0xEF, 0x01]
               },
               {
                   count_test_random,
                   Count_t { value: 0x1EADBEFF },
                   le = [0xFF, 0xBE, 0xAD, 0x1E],
                   be = [0x1E, 0xAD, 0xBE, 0xFF]
               }
);
