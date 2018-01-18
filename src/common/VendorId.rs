#[derive(PartialOrd, PartialEq, Ord, Eq)]
struct VendorId_t {
    vendorId: [u8; 2]
}

const VENDOR_UNKNOWN: VendorId_t = VendorId_t { vendorId: [0x00; 2] };
