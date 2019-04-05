use crate::common::bit_set::BitSetRef;
use num_traits::{NumCast, PrimInt};
use speedy::{Context, Readable, Reader, Writable, Writer};
use std::marker::PhantomData;

#[derive(Debug, PartialEq)]
pub struct RangedBitSet<B, R = B>
where
    B: From<R> + Copy + Clone,
    R: From<B> + PrimInt,
{
    base: B,
    set: BitSetRef,
    repr: PhantomData<R>,
}

impl<B, R> RangedBitSet<B, R>
where
    B: From<R> + Copy + Clone,
    R: From<B> + PrimInt,
{
    fn normalize(&self, value: B) -> Option<usize> {
        let base: R = std::convert::From::from(self.base);
        let value: R = std::convert::From::from(value);

        value
            .checked_sub(&base)
            .and_then(|diff| NumCast::from(diff))
            .and_then(|normalized: u8| Some(std::convert::From::from(normalized)))
    }

    pub fn new(base: B) -> RangedBitSet<B, R> {
        RangedBitSet {
            base: base,
            set: BitSetRef::new(),
            repr: PhantomData,
        }
    }

    pub fn insert(&mut self, value: B) -> bool {
        match self.normalize(value) {
            Some(normalized) => self.set.insert(normalized),
            None => false,
        }
    }
}

impl<'a, C: Context, B, V> Readable<'a, C> for RangedBitSet<B, V>
where
    B: From<V> + speedy::Readable<'a, C> + Copy + Clone,
    V: From<B> + PrimInt,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, std::io::Error> {
        let base: B = reader.read_value()?;
        let set = reader.read_value()?;
        Ok(RangedBitSet {
            base: base,
            set: set,
            repr: PhantomData,
        })
    }
}

impl<C: Context, B, V> Writable<C> for RangedBitSet<B, V>
where
    B: From<V> + speedy::Writable<C> + Copy + Clone,
    V: From<B> + PrimInt,
{
    #[inline]
    fn write_to<'a, T: ?Sized + Writer<'a, C>>(
        &'a self,
        writer: &mut T,
    ) -> Result<(), std::io::Error> {
        writer.write_value(&self.base)?;
        writer.write_value(&self.set)?;
        Ok(())
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
