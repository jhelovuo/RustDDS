use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Visitor};
use std::fmt;

/// Identifies the endianness used to encapsulate the Submessage, the
/// presence of optional elements with in the Submessage, and possibly
/// modifies the interpretation of the Submessage. There are
/// 8 possible flags. The last flag (index 7) identifies the
/// endianness used to encapsulate the Submessage. The remaining
/// flags are interpreted differently depending on the kind
/// of Submessage and are described separately for each Submessage.
#[derive(PartialOrd, PartialEq, Ord, Eq)]
pub struct SubmessageFlag {
    pub flags: [bool;8]
}

impl SubmessageFlag {
    pub fn new() -> SubmessageFlag {
        SubmessageFlag {
            flags: [false;8]
        }
    }
}

impl Serialize for SubmessageFlag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut result: u8 = 0_u8;
        if self.flags[7] { result |= 0b0000_0001 };
        if self.flags[6] { result |= 0b0000_0010 };
        if self.flags[5] { result |= 0b0000_0100 };
        if self.flags[4] { result |= 0b0000_1000 };
        if self.flags[3] { result |= 0b0001_0000 };
        if self.flags[2] { result |= 0b0010_0000 };
        if self.flags[1] { result |= 0b0100_0000 };
        if self.flags[0] { result |= 0b1000_0000 };
        serializer.serialize_u8(result)
    }
}

impl<'de> Deserialize<'de> for SubmessageFlag {
    fn deserialize<D>(deserializer: D) -> Result<SubmessageFlag, D::Error>
        where D: Deserializer<'de>
    {
        struct SubmessageFlagVisitor;
        impl<'de> Visitor<'de> for SubmessageFlagVisitor {
            type Value = SubmessageFlag;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("byte")
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E> {
                Ok(SubmessageFlag{ flags: [v & (1 << 7) != 0,
                                           v & (1 << 6) != 0,
                                           v & (1 << 5) != 0,
                                           v & (1 << 4) != 0,
                                           v & (1 << 3) != 0,
                                           v & (1 << 2) != 0,
                                           v & (1 << 1) != 0,
                                           v & (1 << 0) != 0] })
            }
        }
        deserializer.deserialize_u8(SubmessageFlagVisitor)
    }
}
