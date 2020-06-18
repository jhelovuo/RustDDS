use num_derive::{FromPrimitive, NumOps, ToPrimitive};
use speedy::{Readable, Writable};
use std::convert::From;

#[derive(
  Copy,
  Clone,
  Debug,
  Hash,
  PartialOrd,
  PartialEq,
  Ord,
  Eq,
  Readable,
  Writable,
  NumOps,
  FromPrimitive,
  ToPrimitive,
)]
pub struct FragmentNumber(u32);

impl Default for FragmentNumber {
  fn default() -> FragmentNumber {
    FragmentNumber(1)
  }
}

impl From<u32> for FragmentNumber {
  fn from(value: u32) -> Self {
    FragmentNumber(value)
  }
}

impl From<FragmentNumber> for u32 {
  fn from(fragment_number: FragmentNumber) -> Self {
    fragment_number.0
  }
}

checked_impl!(CheckedAdd, checked_add, FragmentNumber);
checked_impl!(CheckedSub, checked_sub, FragmentNumber);
checked_impl!(CheckedMul, checked_mul, FragmentNumber);
checked_impl!(CheckedDiv, checked_div, FragmentNumber);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fragment_number_starts_by_default_from_one() {
    assert_eq!(FragmentNumber::from(1), FragmentNumber::default());
  }

  serialization_test!( type = FragmentNumber,
  {
      fragment_number_zero,
      FragmentNumber::from(0),
      le = [0x00, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x00]
  },
  {
      fragment_number_default,
      FragmentNumber::default(),
      le = [0x01, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x01]
  },
  {
      fragment_number_non_zero,
      FragmentNumber::from(0xDEADBEEF),
      le = [0xEF, 0xBE, 0xAD, 0xDE],
      be = [0xDE, 0xAD, 0xBE, 0xEF]
  });
}
