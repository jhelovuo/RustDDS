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
pub struct FragmentNumber_t(u32);

impl Default for FragmentNumber_t {
  fn default() -> FragmentNumber_t {
    FragmentNumber_t(1)
  }
}

impl From<u32> for FragmentNumber_t {
  fn from(value: u32) -> Self {
    FragmentNumber_t(value)
  }
}

impl From<FragmentNumber_t> for u32 {
  fn from(fragment_number: FragmentNumber_t) -> Self {
    fragment_number.0
  }
}

checked_impl!(CheckedAdd, checked_add, FragmentNumber_t);
checked_impl!(CheckedSub, checked_sub, FragmentNumber_t);
checked_impl!(CheckedMul, checked_mul, FragmentNumber_t);
checked_impl!(CheckedDiv, checked_div, FragmentNumber_t);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fragment_number_starts_by_default_from_one() {
    assert_eq!(FragmentNumber_t::from(1), FragmentNumber_t::default());
  }

  serialization_test!( type = FragmentNumber_t,
  {
      fragment_number_zero,
      FragmentNumber_t::from(0),
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
      FragmentNumber_t::from(0xDEADBEEF),
      le = [0xEF, 0xBE, 0xAD, 0xDE],
      be = [0xDE, 0xAD, 0xBE, 0xEF]
  });
}
