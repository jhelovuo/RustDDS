use crate::common::ranged_bit_set::RangedBitSet;
use crate::structure::sequence_number::SequenceNumber;

pub type SequenceNumberSet = RangedBitSet<SequenceNumber>;

#[cfg(test)]
mod tests {
  use super::*;

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
