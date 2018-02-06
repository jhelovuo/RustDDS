/// Identifies the endianness used to encapsulate the Submessage, the
/// presence of optional elements with in the Submessage, and possibly
/// modifies the interpretation of the Submessage. There are
/// 8 possible flags. The first flag (index 0) identifies the
/// endianness used to encapsulate the Submessage. The remaining
/// flags are interpreted differently depending on the kind
/// of Submessage and are described separately for each Submessage.
#[derive(Serialize, Deserialize, PartialOrd, PartialEq, Ord, Eq)]
pub struct SubmessageFlag {
    pub flags: u8
}
