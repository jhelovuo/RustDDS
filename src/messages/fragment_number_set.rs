use crate::common::ranged_bit_set::RangedBitSet;
use crate::messages::fragment_number::FragmentNumber;

pub type FragmentNumberSet = RangedBitSet<FragmentNumber>;

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = FragmentNumberSet,
  {
      fragment_number_set_empty,
      FragmentNumberSet::new(FragmentNumber::from(42)),
      le = [0x2A, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x2A,
            0x00, 0x00, 0x00, 0x00]
  },
  {
      fragment_number_set_manual,
      (|| {
          let mut set = FragmentNumberSet::new(FragmentNumber::from(1000));
          set.insert(FragmentNumber::from(1001));
          set.insert(FragmentNumber::from(1003));
          set.insert(FragmentNumber::from(1004));
          set.insert(FragmentNumber::from(1006));
          set.insert(FragmentNumber::from(1008));
          set.insert(FragmentNumber::from(1010));
          set.insert(FragmentNumber::from(1013));
          set
      })(),
      le = [0xE8, 0x03, 0x00, 0x00,
            0x20, 0x00, 0x00, 0x00,
            0x5A, 0x25, 0x00, 0x00],
      be = [0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x20,
            0x00, 0x00, 0x25, 0x5A]
  });
}
