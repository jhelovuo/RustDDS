#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct FragmentNumber_t {
    pub vendor_id: u32,
}

impl Default for FragmentNumber_t {
    fn default() -> FragmentNumber_t {
        FragmentNumber_t { vendor_id: 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_number_starts_by_default_from_one() {
        assert_eq!(
            FragmentNumber_t { vendor_id: 1 },
            FragmentNumber_t::default()
        );
    }
}
