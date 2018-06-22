#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct FragmentNumber_t {
    pub vendor_id: u32
}

impl Default for FragmentNumber_t {
    fn default() -> FragmentNumber_t {
        FragmentNumber_t {
            vendor_id: 1
        }
    }
}
