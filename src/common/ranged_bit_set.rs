use crate::common::bit_set::BitSetRef;
use num_traits::{CheckedAdd, CheckedSub, NumCast, ToPrimitive};
use speedy_derive::{Readable, Writable};
use std::marker::PhantomData;

#[derive(Debug, PartialEq, Readable, Writable)]
pub struct RangedBitSet<B>
where
    B: CheckedAdd + CheckedSub + ToPrimitive,
{
    base: B,
    set: BitSetRef,
}

impl<B> RangedBitSet<B>
where
    B: CheckedAdd + CheckedSub + ToPrimitive,
{
    fn normalize(&self, value: B) -> Option<usize> {
        value
            .checked_sub(&self.base)
            .and_then(|diff| NumCast::from(diff))
            .and_then(|normalized: u8| Some(std::convert::From::from(normalized)))
    }

    pub fn new(base: B) -> RangedBitSet<B> {
        RangedBitSet {
            base: base,
            set: BitSetRef::new(),
        }
    }

    pub fn insert(&mut self, value: B) -> bool {
        match self.normalize(value) {
            Some(normalized) => self.set.insert(normalized),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_values_in_bit_set_range_can_be_inserted() {
        let mut set: RangedBitSet<i64> = RangedBitSet::new(0);

        assert!(!set.insert(std::i64::MIN));
        assert!(!set.insert(-1));

        assert!(set.insert(0));
        assert!(set.insert(1));
        assert!(set.insert(2));
        assert!(set.insert(255));

        assert!(!set.insert(256));
        assert!(!set.insert(std::i64::MAX));
    }

    serialization_test!( type = RangedBitSet<i64>,
    {
        empty_ranged_bit_set,
        RangedBitSet::new(42),
        le = [0x2A, 0x00, 0x00, 0x00,  // bitmapBase
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00], // numBits
        be = [0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x2A,
              0x00, 0x00, 0x00, 0x00]
    });
}
