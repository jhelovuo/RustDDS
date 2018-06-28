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
