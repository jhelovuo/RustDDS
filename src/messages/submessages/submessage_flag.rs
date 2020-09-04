use speedy::{Endianness, Readable};
use enumflags2::BitFlags;

pub trait FromEndianness {
  fn from_endianness(end: speedy::Endianness) -> Self;
}

macro_rules! submessageflag_impls {
  ($t:ident) => {
    impl FromEndianness for BitFlags<$t> {
      fn from_endianness(end: speedy::Endianness) -> Self {
        if end == Endianness::LittleEndian {
          $t::Endianness.into()
        } else {
          Self::empty()
        }
      }
    }
  };
}

pub fn endianness_flag(flags: u8) -> speedy::Endianness {
  if (flags & 0x01) != 0 {
    Endianness::LittleEndian
  } else {
    Endianness::BigEndian
  }
}

/// Identifies the endianness used to encapsulate the Submessage, the
/// presence of optional elements with in the Submessage, and possibly
/// modifies the interpretation of the Submessage. There are
/// 8 possible flags. The first flag (index 0) identifies the
/// endianness used to encapsulate the Submessage. The remaining
/// flags are interpreted differently depending on the kind
/// of Submessage and are described separately for each Submessage.

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum ACKNACK_Flags {
  Endianness = 0b01,
  Final = 0b10,
}
submessageflag_impls!(ACKNACK_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum DATA_Flags {
  Endianness = 0b00001,
  InlineQos = 0b00010,
  Data = 0b00100,
  Key = 0b01000,
  NonStandardPayload = 0b10000,
}
submessageflag_impls!(DATA_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum DATAFRAG_Flags {
  Endianness = 0b00001,
  InlineQos = 0b00010,
  Key = 0b00100,
  NonStandardPayload = 0b01000,
}
submessageflag_impls!(DATAFRAG_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum GAP_Flags {
  Endianness = 0b00001,
}
submessageflag_impls!(GAP_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum HEARTBEAT_Flags {
  Endianness = 0b00001,
  Final = 0b00010,
  Liveliness = 0b00100,
}
submessageflag_impls!(HEARTBEAT_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum HEARTBEATFRAG_Flags {
  Endianness = 0b00001,
}
submessageflag_impls!(HEARTBEATFRAG_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum INFODESTINATION_Flags {
  Endianness = 0b00001,
}
submessageflag_impls!(INFODESTINATION_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum INFOREPLY_Flags {
  Endianness = 0b01,
  Multicast = 0b10,
}
submessageflag_impls!(INFOREPLY_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum INFOSOURCE_Flags {
  Endianness = 0b00001,
}
submessageflag_impls!(INFOSOURCE_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum INFOTIMESTAMP_Flags {
  Endianness = 0b01,
  Invalidate = 0b10,
}
submessageflag_impls!(INFOTIMESTAMP_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum PAD_Flags {
  Endianness = 0b00001,
}
submessageflag_impls!(PAD_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum NACKFRAG_Flags {
  Endianness = 0b00001,
}
submessageflag_impls!(NACKFRAG_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[repr(u8)]
pub enum INFOREPLYIP4_Flags {
  Endianness = 0b01,
  Multicast = 0b10,
}
submessageflag_impls!(INFOREPLYIP4_Flags);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn endianness_flag_test() {
    assert_eq!(Endianness::BigEndian, endianness_flag(0x00));
    assert_eq!(Endianness::LittleEndian, endianness_flag(0x01));
  }
}
