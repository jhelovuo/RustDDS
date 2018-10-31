#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct VendorId_t {
    pub vendorId: [u8; 2]
}

pub const VENDOR_UNKNOWN: VendorId_t = VendorId_t { vendorId: [0x00; 2] };


#[cfg(test)]
mod tests {
    use super::*;

    assert_ser_de!({
        vendor_unknown,
        VENDOR_UNKNOWN,
        le = [0x00, 0x00],
        be = [0x00, 0x00]
    });
}
