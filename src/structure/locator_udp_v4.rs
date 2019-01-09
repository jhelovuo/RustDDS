/// Specialization of Locator_t used to hold UDP IPv4 locators using a more
/// compact representation. Equivalent to Locator_t with kind set to
/// LOCATOR_KIND_UDPv4. Need only be able to hold an IPv4 address and a port
/// number.
pub struct LocatorUDPv4_t {
    /// The mapping between the dot-notation “a.b.c.d” of an IPv4 address and its representation as
    /// an unsigned long is as follows:
    /// address = (((a*256 + b)*256) + c)*256 + d
    address: u32,
    port: u32,
}

impl LocatorUDPv4_t {
    pub const LOCATORUDPv4_INVALID: LocatorUDPv4_t = LocatorUDPv4_t {
        address: 0,
        port: 0,
    };
}
