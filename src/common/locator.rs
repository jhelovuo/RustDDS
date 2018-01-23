#[derive(PartialOrd, PartialEq, Ord, Eq)]
pub struct Locator_t {
    pub kind: i64,
    pub port: u64,
    pub address: [u8; 16]
}

pub const LOCATOR_INVALID: Locator_t = Locator_t { kind: LOCATOR_KIND_INVALID,
                                                   port: LOCATOR_PORT_INVALID,
                                                   address: LOCATOR_ADDRESS_INVALID
};

pub const LOCATOR_KIND_INVALID: i64 = -1;
pub const LOCATOR_ADDRESS_INVALID: [u8; 16] = [0x00; 16];
pub const LOCATOR_PORT_INVALID: u64 = 0;
pub const LOCATOR_KIND_RESERVED: i64 = 0;
pub const LOCATOR_KIND_UDPv4: u8 = 1;
pub const LOCATOR_KIND_UDPv6: u8 = 2;
