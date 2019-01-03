#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Writable)]
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

    serialization_test!( type = FragmentNumber_t,
    {
        fragment_number_zero,
        FragmentNumber_t {
            vendor_id: 0
        },
        le = [0x00, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x00]
    },
    {
        fragment_number_default,
        FragmentNumber_t::default(),
        le = [0x01, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x01]
    },
    {
        fragment_number_non_zero,
        FragmentNumber_t {
            vendor_id: 0xDEADBEEF
        },
        le = [0xEF, 0xBE, 0xAD, 0xDE],
        be = [0xDE, 0xAD, 0xBE, 0xEF]
    });
}
