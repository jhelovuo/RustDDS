#[derive(PartialOrd, PartialEq, Ord, Eq)]
pub struct VendorId_t {
    pub vendorId: [u8; 2]
}

pub const VENDOR_UNKNOWN: VendorId_t = VendorId_t { vendorId: [0x00; 2] };
