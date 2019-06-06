use crate::common::bit_set::BitSetRef;
use num_traits::{CheckedAdd, CheckedSub, FromPrimitive, NumCast, ToPrimitive};
use speedy_derive::{Readable, Writable};

#[derive(Debug, PartialEq, Readable, Writable)]
pub struct RangedBitSet<B>
where
    B: CheckedAdd + CheckedSub + ToPrimitive + FromPrimitive,
{
    base: B,
    set: BitSetRef,
}

impl<B> RangedBitSet<B>
where
    B: CheckedAdd + CheckedSub + ToPrimitive + FromPrimitive,
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

pub struct RangedBitSetIntoIter<B>
where
    B: CheckedAdd + CheckedSub + ToPrimitive + FromPrimitive,
{
    base: B,
    iter: std::iter::Enumerate<bit_vec::IntoIter>,
}

impl<B> Iterator for RangedBitSetIntoIter<B>
where
    B: CheckedAdd + CheckedSub + ToPrimitive + FromPrimitive,
{
    type Item = B;

    fn next(&mut self) -> Option<B> {
        loop {
            match self.iter.next() {
                None => return None,
                Some((_, false)) => {}
                Some((i, true)) => {
                    return Some(B::from_usize(i).unwrap().checked_add(&self.base).unwrap());
                }
            }
        }
    }
}

impl<B> IntoIterator for RangedBitSet<B>
where
    B: CheckedAdd + CheckedSub + ToPrimitive + FromPrimitive,
{
    type Item = B;
    type IntoIter = RangedBitSetIntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        RangedBitSetIntoIter {
            base: self.base,
            iter: self
                .set
                .into_bit_set()
                .into_bit_vec()
                .into_iter()
                .enumerate(),
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

    #[test]
    fn into_iter_test() {
        let mut set: RangedBitSet<i64> = RangedBitSet::new(100);

        assert!(set.insert(100));
        assert!(set.insert(101));
        assert!(set.insert(102));
        assert!(set.insert(100 + 255));

        let mut iter = set.into_iter();
        assert_eq!(Some(100), iter.next());
        assert_eq!(Some(101), iter.next());
        assert_eq!(Some(102), iter.next());
        assert_eq!(Some(100 + 255), iter.next());
        assert_eq!(None, iter.next());
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
