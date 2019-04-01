use crate::common::bit_set::BitSetRef;
use speedy::{Context, Readable, Reader, Writable, Writer};
use std::convert::TryFrom;
use std::ops::Sub;

#[derive(Debug, PartialEq)]
pub struct RangedBitSet<B>
where
    B: Sub<Output = B> + Copy + Clone + PartialOrd,
    u8: TryFrom<B>,
{
    base: B,
    set: BitSetRef,
}

impl<B> RangedBitSet<B>
where
    B: Sub<Output = B> + Copy + Clone + PartialOrd,
    u8: TryFrom<B>,
{
    fn normalize(&self, value: B) -> Option<usize> {
        if self.base <= value {
            u8::try_from(value - self.base)
                .ok()
                .and_then(|normalized| Some(usize::from(normalized)))
        } else {
            None
        }
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

impl<'a, C: Context, B> Readable<'a, C> for RangedBitSet<B>
where
    B: speedy::Readable<'a, C> + Copy + Clone + Sub<Output = B> + PartialOrd,
    u8: TryFrom<B>,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, std::io::Error> {
        let base: B = reader.read_value()?;
        let set = reader.read_value()?;
        Ok(RangedBitSet {
            base: base,
            set: set,
        })
    }
}

impl<C: Context, B> Writable<C> for RangedBitSet<B>
where
    B: speedy::Writable<C> + Copy + Clone + Sub<Output = B> + PartialOrd,
    u8: TryFrom<B>,
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
