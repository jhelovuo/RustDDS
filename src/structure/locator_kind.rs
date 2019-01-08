use speedy_derive::{Readable, Writable};

#[derive(Clone, Debug, Eq, PartialEq, Readable, Writable)]
pub struct LocatorKind_t {
    value: i32,
}

impl LocatorKind_t {
    pub const LOCATOR_KIND_INVALID: LocatorKind_t = LocatorKind_t { value: -1 };
    pub const LOCATOR_KIND_RESERVED: LocatorKind_t = LocatorKind_t { value: 0 };
    pub const LOCATOR_KIND_UDPv4: LocatorKind_t = LocatorKind_t { value: 1 };
    pub const LOCATOR_KIND_UDPv6: LocatorKind_t = LocatorKind_t { value: 2 };
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = LocatorKind_t,
        {
            locator_kind_invalid,
            LocatorKind_t::LOCATOR_KIND_INVALID,
            le = [0xFF, 0xFF, 0xFF, 0xFF],
            be = [0xFF, 0xFF, 0xFF, 0xFF]
        },
        {
            locator_kind_reserved,
            LocatorKind_t::LOCATOR_KIND_RESERVED,
            le = [0x00, 0x00, 0x00, 0x00],
            be = [0x00, 0x00, 0x00, 0x00]
        },
        {
            locator_kind_udpv4,
            LocatorKind_t::LOCATOR_KIND_UDPv4,
            le = [0x01, 0x00, 0x00, 0x00],
            be = [0x00, 0x00, 0x00, 0x01]
        },
        {
            locator_kind_udpv6,
            LocatorKind_t::LOCATOR_KIND_UDPv6,
            le = [0x02, 0x00, 0x00, 0x00],
            be = [0x00, 0x00, 0x00, 0x02]
        }
    );
}
