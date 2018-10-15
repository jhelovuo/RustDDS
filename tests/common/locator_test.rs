extern crate rtps;

use self::rtps::common::locator;
use self::rtps::common::locator::{SocketAddr, IpAddr, Ipv4Addr, Ipv6Addr};

#[test]
fn verify_locator_address_invalid() {
    assert_eq!([0x00; 16], locator::LOCATOR_ADDRESS_INVALID);
}

#[test]
fn verify_locator_port_invalid() {
    assert_eq!(0, locator::LOCATOR_PORT_INVALID);
}

#[test]
fn locator_invalid_is_a_concatenation_of_invalid_members() {
    assert_eq!(locator::Locator_t {
        kind: locator::LocatorKind_t::LOCATOR_KIND_INVALID,
        port: locator::LOCATOR_PORT_INVALID,
        address: locator::LOCATOR_ADDRESS_INVALID
    }, locator::LOCATOR_INVALID);
}

assert_ser_de!(
    {
        locator_invalid,
        locator::LOCATOR_INVALID,
        le = [
            0xFF, 0xFF, 0xFF, 0xFF,  // LocatorKind_t::LOCATOR_KIND_INVALID
            0x00, 0x00, 0x00, 0x00,  // Locator_t::LOCATOR_PORT_INVALID,
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[0:3]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
            0x00, 0x00, 0x00, 0x00   // Locator_t::address[12:15]
        ],
        be = [
            0xFF, 0xFF, 0xFF, 0xFF,  // LocatorKind_t::LOCATOR_KIND_UDPv4
            0x00, 0x00, 0x00, 0x00,  // Locator_t::LOCATOR_PORT_INVALID,
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[0:3]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
            0x00, 0x00, 0x00, 0x00   // Locator_t::address[12:15]
        ]
    },
    {
        locator_invalid_ipv6,
        locator::Locator_t::from(SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), 7171)),
        le = [
            0xFF, 0xFF, 0xFF, 0xFF,  // LocatorKind_t::LOCATOR_KIND_INVALID
            0x03, 0x1C, 0x00, 0x00,  // Locator_t::port(7171),
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[0:3]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
            0x00, 0x00, 0x00, 0x00   // Locator_t::address[12:15]
        ],
        be = [
            0xFF, 0xFF, 0xFF, 0xFF,  // LocatorKind_t::LOCATOR_KIND_INVALID
            0x00, 0x00, 0x1C, 0x03,  // Locator_t::port(7171),
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[0:3]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
            0x00, 0x00, 0x00, 0x00   // Locator_t::address[12:15]
        ]
    },
    {
        locator_localhost_ipv4,
        locator::Locator_t::from(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)),
        le = [
            0x01, 0x00, 0x00, 0x00,  // LocatorKind_t::LOCATOR_KIND_UDPv4
            0x90, 0x1F, 0x00, 0x00,  // Locator_t::port(8080),
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[0:3]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
            0x7F, 0x00, 0x00, 0x01   // Locator_t::address[12:15]
        ],
        be = [
            0x00, 0x00, 0x00, 0x01,  // LocatorKind_t::LOCATOR_KIND_UDPv4
            0x00, 0x00, 0x1F, 0x90,  // Locator_t::port(8080),
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[0:3]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
            0x7F, 0x00, 0x00, 0x01   // Locator_t::address[12:15]
        ]
    },
    {
        locator_ipv6,
        locator::Locator_t::from(SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0xFF00, 0x4501, 0, 0, 0, 0, 0, 0x0032)), 7171)),
        le = [
            0x02, 0x00, 0x00, 0x00,  // LocatorKind_t::LOCATOR_KIND_UDPv6
            0x03, 0x1C, 0x00, 0x00,  // Locator_t::port(7171),
            0xFF, 0x00, 0x45, 0x01,  // Locator_t::address[0:3]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
            0x00, 0x00, 0x00, 0x32   // Locator_t::address[12:15]
        ],
        be = [
            0x00, 0x00, 0x00, 0x02,  // LocatorKind_t::LOCATOR_KIND_UDPv6
            0x00, 0x00, 0x1C, 0x03,  // Locator_t::port(7171),
            0xFF, 0x00, 0x45, 0x01,  // Locator_t::address[0:3]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
            0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
            0x00, 0x00, 0x00, 0x32   // Locator_t::address[12:15]
        ]
    }
);
