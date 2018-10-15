extern crate rtps;

use self::rtps::common::guid_prefix::{GuidPrefix_t, GUIDPREFIX_UNKNOWN};

assert_ser_de!({
                   guid_prefix_unknown,
                   GUIDPREFIX_UNKNOWN,
                   le = [0x00; 12],
                   be = [0x00; 12]
               },
               {
                   guid_prefix_default,
                   GuidPrefix_t::default(),
                   le = [0x00; 12],
                   be = [0x00; 12]
               },
               {
                   guid_prefix_endianness_insensitive,
                   GuidPrefix_t {
                       entityKey: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
                                   0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]
                   },
                   le = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
                         0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB],
                   be = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
                         0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]
               }
);
