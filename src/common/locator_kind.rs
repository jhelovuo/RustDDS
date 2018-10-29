use crate::enum_number;

enum_number_i32!(LocatorKind_t {
    LOCATOR_KIND_INVALID = -1,
    LOCATOR_KIND_RESERVED = 0,
    LOCATOR_KIND_UDPv4 = 1,
    LOCATOR_KIND_UDPv6 = 2,
});
