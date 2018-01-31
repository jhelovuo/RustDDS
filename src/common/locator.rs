#[derive(PartialOrd, PartialEq, Ord, Eq)]
pub struct Locator_t {
    pub kind: i32,
    pub port: u32,
    pub address: [u8; 16]
}

type LocatorList_t = Vec<Locator_t>;

pub const LOCATOR_INVALID: Locator_t = Locator_t { kind: LOCATOR_KIND_INVALID,
                                                   port: LOCATOR_PORT_INVALID,
                                                   address: LOCATOR_ADDRESS_INVALID
};

pub const LOCATOR_KIND_INVALID: i32 = -1;
pub const LOCATOR_ADDRESS_INVALID: [u8; 16] = [0x00; 16];
pub const LOCATOR_PORT_INVALID: u32 = 0;
pub const LOCATOR_KIND_RESERVED: i32 = 0;
pub const LOCATOR_KIND_UDPv4: u8 = 1;
pub const LOCATOR_KIND_UDPv6: u8 = 2;
