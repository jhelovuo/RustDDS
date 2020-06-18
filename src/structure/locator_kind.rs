use speedy::{Readable, Writable};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Readable, Writable)]
pub struct LocatorKind {
  value: i32,
}

impl LocatorKind {
  pub const LOCATOR_KIND_INVALID: LocatorKind = LocatorKind { value: -1 };
  pub const LOCATOR_KIND_RESERVED: LocatorKind = LocatorKind { value: 0 };
  pub const LOCATOR_KIND_UDPv4: LocatorKind = LocatorKind { value: 1 };
  pub const LOCATOR_KIND_UDPv6: LocatorKind = LocatorKind { value: 2 };
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
          LocatorKind::LOCATOR_KIND_UDPv4,
          le = [0x01, 0x00, 0x00, 0x00],
          be = [0x00, 0x00, 0x00, 0x01]
      },
      {
          locator_kind_udpv6,
          LocatorKind::LOCATOR_KIND_UDPv6,
          le = [0x02, 0x00, 0x00, 0x00],
          be = [0x00, 0x00, 0x00, 0x02]
      }
  );
}
