use std::convert::From;
pub use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use speedy::{Context, Readable, Reader, Writable, Writer};
use serde::{Deserialize, Serialize};

use super::parameter_id::ParameterId;

#[derive(
  Copy,
  Clone,
  Debug,
  Eq,
  PartialEq,
  PartialOrd,
  Ord,
  Hash,
  Readable,
  Writable,
  Serialize,
  Deserialize,
)]
pub struct LocatorKind {
  value: i32,
}

impl LocatorKind {
  pub const LOCATOR_KIND_INVALID: LocatorKind = LocatorKind { value: -1 };
  pub const LOCATOR_KIND_RESERVED: LocatorKind = LocatorKind { value: 0 };
  pub const LOCATOR_KIND_UDP_V4: LocatorKind = LocatorKind { value: 1 };
  pub const LOCATOR_KIND_UDP_V6: LocatorKind = LocatorKind { value: 2 };
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Locator {
  pub kind: LocatorKind,
  pub port: u32,
  pub address: [u8; 16],
}

pub type LocatorList = Vec<Locator>;

impl Locator {
  pub const LOCATOR_INVALID: Locator = Locator {
    kind: LocatorKind::LOCATOR_KIND_INVALID,
    port: Locator::LOCATOR_PORT_INVALID,
    address: Locator::LOCATOR_ADDRESS_INVALID,
  };

  pub const LOCATOR_ADDRESS_INVALID: [u8; 16] = [0x00; 16];
  pub const LOCATOR_PORT_INVALID: u32 = 0;

  pub fn to_socket_address(self) -> SocketAddr {
    SocketAddr::from(self)
  }

  pub fn is_udp(&self) -> bool {
    self.kind == LocatorKind::LOCATOR_KIND_UDP_V4 || self.kind == LocatorKind::LOCATOR_KIND_UDP_V6
  }
}

impl Default for Locator {
  fn default() -> Self {
    Locator::LOCATOR_INVALID
  }
}

impl From<SocketAddr> for Locator {
  fn from(socket_address: SocketAddr) -> Self {
    Locator {
      kind: if socket_address.ip().is_unspecified() {
        LocatorKind::LOCATOR_KIND_INVALID
      } else if socket_address.ip().is_ipv4() {
        LocatorKind::LOCATOR_KIND_UDP_V4
      } else {
        LocatorKind::LOCATOR_KIND_UDP_V6
      },
      port: u32::from(socket_address.port()),
      address: match socket_address.ip() {
        IpAddr::V4(ip4) => ip4.to_ipv6_compatible().octets(),
        IpAddr::V6(ip6) => ip6.octets(),
      },
    }
  }
}

impl From<Locator> for SocketAddr {
  fn from(locator: Locator) -> Self {
    match locator.kind {
      LocatorKind::LOCATOR_KIND_UDP_V4 => SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(
          locator.address[12],
          locator.address[13],
          locator.address[14],
          locator.address[15],
        )),
        locator.port as u16,
      ),
      LocatorKind::LOCATOR_KIND_UDP_V6 => SocketAddr::new(
        IpAddr::V6(Ipv6Addr::from(locator.address)),
        locator.port as u16,
      ),
      _ => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    }
  }
}

impl<'a, C: Context> Readable<'a, C> for Locator {
  #[inline]
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let mut locator = Locator {
      kind: reader.read_value()?,
      port: reader.read_value()?,
      ..Locator::default()
    };
    for i in 0..locator.address.len() {
      locator.address[i] = reader.read_u8()?;
    }
    Ok(locator)
  }
}

impl<C: Context> Writable<C> for Locator {
  #[inline]
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    writer.write_value(&self.kind)?;
    writer.write_value(&self.port)?;
    for elem in &self.address {
      writer.write_u8(*elem)?;
    }
    Ok(())
  }
}

#[derive(Serialize, Deserialize)]
pub struct LocatorData {
  parameter_id: ParameterId,
  parameter_length: u16,
  locator: Locator,
}

impl LocatorData {
  pub fn from(locator: &Locator, parameter_id: ParameterId) -> LocatorData {
    LocatorData {
      parameter_id,
      parameter_length: 24,
      locator: *locator,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = LocatorKind,
    {
        locator_kind_invalid,
        LocatorKind::LOCATOR_KIND_INVALID,
        le = [0xFF, 0xFF, 0xFF, 0xFF],
        be = [0xFF, 0xFF, 0xFF, 0xFF]
    },
    {
        locator_kind_reserved,
        LocatorKind::LOCATOR_KIND_RESERVED,
        le = [0x00, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x00]
    },
    {
        locator_kind_udpv4,
        LocatorKind::LOCATOR_KIND_UDP_V4,
        le = [0x01, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x01]
    },
    {
        locator_kind_udpv6,
        LocatorKind::LOCATOR_KIND_UDP_V6,
        le = [0x02, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x02]
    }
  );

  #[test]
  fn verify_locator_address_invalid() {
    assert_eq!([0x00; 16], Locator::LOCATOR_ADDRESS_INVALID);
  }

  #[test]
  fn verify_locator_port_invalid() {
    assert_eq!(0, Locator::LOCATOR_PORT_INVALID);
  }

  #[test]
  fn locator_invalid_is_a_concatenation_of_invalid_members() {
    assert_eq!(
      Locator {
        kind: LocatorKind::LOCATOR_KIND_INVALID,
        port: Locator::LOCATOR_PORT_INVALID,
        address: Locator::LOCATOR_ADDRESS_INVALID
      },
      Locator::LOCATOR_INVALID
    );
  }

  macro_rules! conversion_test {
        ($({ $name:ident, $left:expr, $right:expr }),+) => {
            $(mod $name {
                use super::*;

                #[test]
                fn left_into_right() {
                    assert_eq!($right, ($left).into())
                }

                #[test]
                fn right_into_left() {
                    assert_eq!($left, ($right).into());
                }
            })+
        }
    }

  conversion_test!(
  {
      invalid_into_unspecified,
      Locator::LOCATOR_INVALID,
      SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0)
  },
  {
      non_empty_ipv4,
      Locator {
          kind: LocatorKind::LOCATOR_KIND_UDP_V4,
          address: [
              0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
              0x7F, 0x00, 0x00, 0x01
          ],
          port: 8080
      },
      SocketAddr::new(
          IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
          8080
      )
  },
  {
      non_empty_ipv6,
      Locator {
          kind: LocatorKind::LOCATOR_KIND_UDP_V6,
          address: [
              0xFF, 0x00, 0x45, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x32
          ],
          port: 7171
      },
      SocketAddr::new(
          IpAddr::V6(Ipv6Addr::new(0xFF00, 0x4501, 0, 0, 0, 0, 0, 0x0032)),
          7171
      )
  });

  serialization_test!( type = Locator,
      {
          locator_invalid,
          Locator::LOCATOR_INVALID,
          le = [
              0xFF, 0xFF, 0xFF, 0xFF,  // LocatorKind_t::LOCATOR_KIND_INVALID
              0x00, 0x00, 0x00, 0x00,  // Locator_t::LOCATOR_PORT_INVALID,
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[0:3]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
              0x00, 0x00, 0x00, 0x00   // Locator_t::address[12:15]
          ],
          be = [
              0xFF, 0xFF, 0xFF, 0xFF,  // LocatorKind_t::LOCATOR_KIND_UDP_V4
              0x00, 0x00, 0x00, 0x00,  // Locator_t::LOCATOR_PORT_INVALID,
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[0:3]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
              0x00, 0x00, 0x00, 0x00   // Locator_t::address[12:15]
          ]
      },
      {
          locator_invalid_ipv6,
          Locator::from(SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), 7171)),
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
          Locator::from(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)),
          le = [
              0x01, 0x00, 0x00, 0x00,  // LocatorKind_t::LOCATOR_KIND_UDP_V4
              0x90, 0x1F, 0x00, 0x00,  // Locator_t::port(8080),
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[0:3]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
              0x7F, 0x00, 0x00, 0x01   // Locator_t::address[12:15]
          ],
          be = [
              0x00, 0x00, 0x00, 0x01,  // LocatorKind_t::LOCATOR_KIND_UDP_V4
              0x00, 0x00, 0x1F, 0x90,  // Locator_t::port(8080),
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[0:3]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
              0x7F, 0x00, 0x00, 0x01   // Locator_t::address[12:15]
          ]
      },
      {
          locator_ipv6,
          Locator::from(SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0xFF00, 0x4501, 0, 0, 0, 0, 0, 0x0032)), 7171)),
          le = [
              0x02, 0x00, 0x00, 0x00,  // LocatorKind_t::LOCATOR_KIND_UDP_V6
              0x03, 0x1C, 0x00, 0x00,  // Locator_t::port(7171),
              0xFF, 0x00, 0x45, 0x01,  // Locator_t::address[0:3]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
              0x00, 0x00, 0x00, 0x32   // Locator_t::address[12:15]
          ],
          be = [
              0x00, 0x00, 0x00, 0x02,  // LocatorKind_t::LOCATOR_KIND_UDP_V6
              0x00, 0x00, 0x1C, 0x03,  // Locator_t::port(7171),
              0xFF, 0x00, 0x45, 0x01,  // Locator_t::address[0:3]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[4:7]
              0x00, 0x00, 0x00, 0x00,  // Locator_t::address[8:11]
              0x00, 0x00, 0x00, 0x32   // Locator_t::address[12:15]
          ]
      }
  );
}
