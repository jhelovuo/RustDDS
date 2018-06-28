use std::net::{SocketAddr, IpAddr, Ipv4Addr, Ipv6Addr};
use std::convert::{From, Into};

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct Locator_t {
    pub kind: LocatorKind_t,
    pub port: u32,
    pub address: [u8; 16]
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
#[repr(i32)]
pub enum LocatorKind_t {
    LOCATOR_KIND_INVALID = -1,
    LOCATOR_KIND_RESERVED = 0,
    LOCATOR_KIND_UDPv4 = 1,
    LOCATOR_KIND_UDPv6 = 2
}

pub type LocatorList_t = Vec<Locator_t>;

pub const LOCATOR_INVALID: Locator_t = Locator_t { kind: LocatorKind_t::LOCATOR_KIND_INVALID,
                                                   port: LOCATOR_PORT_INVALID,
                                                   address: LOCATOR_ADDRESS_INVALID
};

pub const LOCATOR_ADDRESS_INVALID: [u8; 16] = [0x00; 16];
pub const LOCATOR_PORT_INVALID: u32 = 0;

impl From<SocketAddr> for Locator_t {
    fn from(socket_address: SocketAddr) -> Self {
        Locator_t {
            kind: match socket_address.ip().is_unspecified() {
                true => LocatorKind_t::LOCATOR_KIND_INVALID,
                false => match socket_address.ip().is_ipv4() {
                    true => LocatorKind_t::LOCATOR_KIND_UDPv4,
                    false => LocatorKind_t::LOCATOR_KIND_UDPv6
                }
            },
            port: socket_address.port() as u32,
            address: match socket_address.ip() {
                IpAddr::V4(ip4) => ip4.to_ipv6_compatible().octets(),
                IpAddr::V6(ip6) => ip6.octets()
            }
        }
    }
}

impl Into<SocketAddr> for Locator_t {
    fn into(self) -> SocketAddr {
        match self.kind {
            LocatorKind_t::LOCATOR_KIND_UDPv4 =>
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(self.address[12],
                                                         self.address[13],
                                                         self.address[14],
                                                         self.address[15])), self.port as u16),
            _ =>
                SocketAddr::new(IpAddr::V6(Ipv6Addr::from(self.address)), self.port as u16)
        }
    }
}
