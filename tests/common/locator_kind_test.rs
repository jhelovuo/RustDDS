extern crate rtps;

use self::rtps::common::locator_kind;

assert_ser_de!({
                   locator_kind_invalid,
                   locator_kind::LocatorKind_t::LOCATOR_KIND_INVALID,
                   le = [0xFF, 0xFF, 0xFF, 0xFF],
                   be = [0xFF, 0xFF, 0xFF, 0xFF]
               },
               {
                   locator_kind_reserved,
                   locator_kind::LocatorKind_t::LOCATOR_KIND_RESERVED,
                   le = [0x00, 0x00, 0x00, 0x00],
                   be = [0x00, 0x00, 0x00, 0x00]
               },
               {
                   locator_kind_udpv4,
                   locator_kind::LocatorKind_t::LOCATOR_KIND_UDPv4,
                   le = [0x01, 0x00, 0x00, 0x00],
                   be = [0x00, 0x00, 0x00, 0x01]
               },
               {
                   locator_kind_udpv6,
                   locator_kind::LocatorKind_t::LOCATOR_KIND_UDPv6,
                   le = [0x02, 0x00, 0x00, 0x00],
                   be = [0x00, 0x00, 0x00, 0x02]
               }
);
