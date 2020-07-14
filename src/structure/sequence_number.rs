use num_derive::{FromPrimitive, NumOps, ToPrimitive};
use speedy::{Context, Readable, Reader, Writable, Writer};
use std::convert::From;
use std::mem::size_of;

use crate::common::ranged_bit_set::RangedBitSet;

#[derive(
  Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, NumOps, 
  FromPrimitive, ToPrimitive,
)]
pub struct SequenceNumber(i64);

impl SequenceNumber {
  pub const SEQUENCENUMBER_UNKNOWN: SequenceNumber = 
    SequenceNumber((std::u32::MAX as i64) << 32);
}

impl From<i64> for SequenceNumber {
  fn from(value: i64) -> Self {
    SequenceNumber(value)
  }
}

impl From<SequenceNumber> for i64 {
  fn from(sequence_number: SequenceNumber) -> Self {
    sequence_number.0
  }
}

checked_impl!(CheckedAdd, checked_add, SequenceNumber);
checked_impl!(CheckedSub, checked_sub, SequenceNumber);
checked_impl!(CheckedMul, checked_mul, SequenceNumber);
checked_impl!(CheckedDiv, checked_div, SequenceNumber);

impl<'a, C: Context> Readable<'a, C> for SequenceNumber {
  #[inline]
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let high: i32 = reader.read_value()?;
    let low: u32 = reader.read_value()?;

    Ok(SequenceNumber(((i64::from(high)) << 32) + i64::from(low)))
  }

  #[inline]
  fn minimum_bytes_needed() -> usize {
    size_of::<Self>()
  }
}

impl<C: Context> Writable<C> for SequenceNumber {
  #[inline]
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    writer.write_i32((self.0 >> 32) as i32)?;
    writer.write_u32(self.0 as u32)?;
    Ok(())
  }
}

impl Default for SequenceNumber {
  fn default() -> SequenceNumber {
    SequenceNumber(1)
  }
}

pub type SequenceNumberSet = RangedBitSet<SequenceNumber>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sequence_number_starts_by_default_from_one() {
    assert_eq!(SequenceNumber::from(1), SequenceNumber::default());
  }

  serialization_test!( type = SequenceNumber,
  {
      sequence_number_default,
      SequenceNumber::default(),
      le = [0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
  },
  {
      sequence_number_unknown,
      SequenceNumber::SEQUENCENUMBER_UNKNOWN,
      le = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00],
      be = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00]
  },
  {
      sequence_number_non_zero,
      SequenceNumber::from(0x0011223344556677),
      le = [0x33, 0x22, 0x11, 0x00, 0x77, 0x66, 0x55, 0x44],
      be = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]
  });

  serialization_test!( type = SequenceNumberSet,
  {
      sequence_number_set_empty,
      SequenceNumberSet::new(SequenceNumber::from(42)),
      le = [0x00, 0x00, 0x00, 0x00,  // bitmapBase
            0x2A, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00], // numBits
      be = [0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x2A,
            0x00, 0x00, 0x00, 0x00]
  },
  {
      sequence_number_set_manual,
      (|| {
          let mut set = SequenceNumberSet::new(SequenceNumber::from(1000));
          set.insert(SequenceNumber::from(1001));
          set.insert(SequenceNumber::from(1003));
          set.insert(SequenceNumber::from(1004));
          set.insert(SequenceNumber::from(1006));
          set.insert(SequenceNumber::from(1008));
          set.insert(SequenceNumber::from(1010));
          set.insert(SequenceNumber::from(1013));
          set
      })(),
      le = [0x00, 0x00, 0x00, 0x00,
            0xE8, 0x03, 0x00, 0x00,
            0x20, 0x00, 0x00, 0x00,
            0x5A, 0x25, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x20,
            0x00, 0x00, 0x25, 0x5A]
  });
}
