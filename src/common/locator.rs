use speedy::{Context, Readable, Reader, Writable, Writer};
use std::convert::{From, Into};
use std::io::Result;
use std::mem::size_of;
pub use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub use crate::common::locator_kind::LocatorKind_t;

#[derive(Debug, PartialEq, Eq)]
pub struct Locator_t {
    pub kind: LocatorKind_t,
    pub port: u32,
    pub address: [u8; 16],
}

pub type LocatorList_t = Vec<Locator_t>;

pub const LOCATOR_INVALID: Locator_t = Locator_t {
    kind: LocatorKind_t::LOCATOR_KIND_INVALID,
    port: LOCATOR_PORT_INVALID,
    address: LOCATOR_ADDRESS_INVALID,
};

pub const LOCATOR_ADDRESS_INVALID: [u8; 16] = [0x00; 16];
pub const LOCATOR_PORT_INVALID: u32 = 0;

impl Default for Locator_t {
    fn default() -> Self {
        LOCATOR_INVALID
    }
}

impl From<SocketAddr> for Locator_t {
    fn from(socket_address: SocketAddr) -> Self {
        Locator_t {
            kind: match socket_address.ip().is_unspecified() {
                true => LocatorKind_t::LOCATOR_KIND_INVALID,
                false => match socket_address.ip().is_ipv4() {
                    true => LocatorKind_t::LOCATOR_KIND_UDPv4,
                    false => LocatorKind_t::LOCATOR_KIND_UDPv6,
                },
            },
            port: socket_address.port() as u32,
            address: match socket_address.ip() {
                IpAddr::V4(ip4) => ip4.to_ipv6_compatible().octets(),
                IpAddr::V6(ip6) => ip6.octets(),
            },
        }
    }
}

impl<'a, C: Context> Readable<'a, C> for Locator_t {
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self> {
        let mut locator = Locator_t::default();
        locator.kind = reader.read_value()?;
        locator.port = reader.read_value()?;
        for i in 0..locator.address.len() {
            locator.address[i] = reader.read_u8()?;
        }
        Ok(locator)
    }
}

impl<C: Context> Writable<C> for Locator_t {
    #[inline]
    fn write_to<'a, T: ?Sized + Writer<'a, C>>(&'a self, writer: &mut T) -> Result<()> {
        writer.write_value(&self.kind)?;
        writer.write_value(&self.port)?;
        for elem in &self.address {
            writer.write_u8(*elem)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_locator_address_invalid() {
        assert_eq!([0x00; 16], LOCATOR_ADDRESS_INVALID);
    }

    #[test]
    fn verify_locator_port_invalid() {
        assert_eq!(0, LOCATOR_PORT_INVALID);
    }

    #[test]
    fn locator_invalid_is_a_concatenation_of_invalid_members() {
        assert_eq!(
            Locator_t {
                kind: LocatorKind_t::LOCATOR_KIND_INVALID,
                port: LOCATOR_PORT_INVALID,
                address: LOCATOR_ADDRESS_INVALID
            },
            LOCATOR_INVALID
        );
    }

    serialization_test!( type = Locator_t,
        {
            locator_invalid,
            LOCATOR_INVALID,
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
            Locator_t::from(SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), 7171)),
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
            Locator_t::from(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)),
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
            Locator_t::from(SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0xFF00, 0x4501, 0, 0, 0, 0, 0, 0x0032)), 7171)),
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
}
