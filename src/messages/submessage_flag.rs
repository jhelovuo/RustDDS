use speedy::{Endianness, Readable, Writable};

/// Identifies the endianness used to encapsulate the Submessage, the
/// presence of optional elements with in the Submessage, and possibly
/// modifies the interpretation of the Submessage. There are
/// 8 possible flags. The first flag (index 0) identifies the
/// endianness used to encapsulate the Submessage. The remaining
/// flags are interpreted differently depending on the kind
/// of Submessage and are described separately for each Submessage.
#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Writable)]
pub struct SubmessageFlag {
    pub flags: u8,
}

impl SubmessageFlag {
    /// Indicates endianness
    pub fn endianness_flag(&self) -> speedy::Endianness {
        if self.is_flag_set(0x01) {
            Endianness::LittleEndian
        } else {
            Endianness::BigEndian
        }
    }

    pub fn set_flag(&mut self, bit: u8) {
        self.flags |= bit;
    }
    pub fn clear_flag(&mut self, bit: u8) {
        self.flags &= !bit;
    }
    pub fn is_flag_set(&self, bit: u8) -> bool {
        self.flags & bit != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_bits_order() {
        let submessage_flag = SubmessageFlag {
            flags: 0b10110100_u8,
        };

        assert!(!submessage_flag.is_flag_set(0b0000_0001));
        assert!(!submessage_flag.is_flag_set(0b0000_0010));
        assert!(submessage_flag.is_flag_set(0b0000_0100));
        assert!(!submessage_flag.is_flag_set(0b0000_1000));
        assert!(submessage_flag.is_flag_set(0b0001_0000));
        assert!(submessage_flag.is_flag_set(0b0010_0000));
        assert!(!submessage_flag.is_flag_set(0b0100_0000));
        assert!(submessage_flag.is_flag_set(0b1000_0000));
    }

    #[test]
    fn helper_functions_test() {
        for x in 0..7 {
            let mut flags = SubmessageFlag { flags: 0x00 };
            let bit = u8::from(2).pow(x);

            assert!(!flags.is_flag_set(bit));
            flags.set_flag(bit);
            assert!(flags.is_flag_set(bit));
            flags.clear_flag(bit);
            assert!(!flags.is_flag_set(bit));
        }
    }

    serialization_test!(type = SubmessageFlag,
    {
        submessage_flag,
        SubmessageFlag { flags: 0b10110100_u8 },
        le = [0b10110100_u8],
        be = [0b10110100_u8]
    });
}
