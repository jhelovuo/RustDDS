/// Identifies the endianness used to encapsulate the Submessage, the
/// presence of optional elements with in the Submessage, and possibly
/// modifies the interpretation of the Submessage. There are
/// 8 possible flags. The first flag (index 0) identifies the
/// endianness used to encapsulate the Submessage. The remaining
/// flags are interpreted differently depending on the kind
/// of Submessage and are described separately for each Submessage.
#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Ord, Eq)]
pub struct SubmessageFlag {
    pub flags: u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_bits_order() {
        let submessage_flag = SubmessageFlag { flags: 0b10110100_u8 };

        assert!(submessage_flag.flags & 0x01 == 0);
        assert!(submessage_flag.flags & (1 << 0) == 0);

        assert!(submessage_flag.flags & 0x80 != 0);
        assert!(submessage_flag.flags & (1 << 7) != 0);
    }

    assert_ser_de!({
        submessage_flag,
                   SubmessageFlag { flags: 0b10110100_u8 },
        le = [0b10110100_u8],
        be = [0b10110100_u8]
    });
}
