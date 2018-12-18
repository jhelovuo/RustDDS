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
    /// Indicates endianness. Returns true if big-endian, false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.flags & 0x01 != 0
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

        assert!(submessage_flag.flags & 0x01 == 0);
        assert!(submessage_flag.flags & (1 << 0) == 0);

        assert!(submessage_flag.flags & 0x80 != 0);
        assert!(submessage_flag.flags & (1 << 7) != 0);
    }

    #[test]
    fn endianness_flag() {
        let flag_with_big_endian = SubmessageFlag { flags: 0x01 };
        assert!(flag_with_big_endian.endianness_flag());

        let flag_with_little_endian = SubmessageFlag { flags: 0x00 };
        assert!(!flag_with_little_endian.endianness_flag());
    }

    serialization_test!(type = SubmessageFlag,
        {
            submessage_flag,
            SubmessageFlag { flags: 0b10110100_u8 },
            le = [0b10110100_u8],
            be = [0b10110100_u8]
        });
}
