use bit_set::BitSet;
use bit_vec::{BitBlock, BitVec};
use crate::message::Validity;
use speedy::{Context, Readable, Reader, Writable, Writer};
use std::cmp::Ordering;
use std::convert::{From, Into};
use std::io::Result;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::Add;
use std::ops::Sub;
use std::ops::{Deref, DerefMut};
use std::{cmp, fmt};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Readable, Writable)]
pub struct SequenceNumber_t {
    pub high: i32,
    pub low: u32,
}

pub const SEQUENCENUMBER_UNKNOWN: SequenceNumber_t = SequenceNumber_t { high: -1, low: 0 };

impl SequenceNumber_t {
    pub fn value(&self) -> u64 {
        ((self.high as u64) << 32) + self.low as u64
    }
}

impl Default for SequenceNumber_t {
    fn default() -> SequenceNumber_t {
        SequenceNumber_t { high: 0, low: 1 }
    }
}

impl Add<u32> for SequenceNumber_t {
    type Output = SequenceNumber_t;

    fn add(self, other: u32) -> SequenceNumber_t {
        let (new_low, overflow) = self.low.overflowing_add(other);

        SequenceNumber_t {
            high: match overflow {
                true => self.high + 1,
                false => self.high,
            },
            low: new_low,
        }
    }
}

impl Add<SequenceNumber_t> for SequenceNumber_t {
    type Output = SequenceNumber_t;

    fn add(self, other: SequenceNumber_t) -> SequenceNumber_t {
        let (new_low, overflow) = self.low.overflowing_add(other.low);
        SequenceNumber_t {
            high: match overflow {
                true => self.high + other.high + 1,
                false => self.high + other.high,
            },
            low: new_low,
        }
    }
}

impl Sub<u32> for SequenceNumber_t {
    type Output = SequenceNumber_t;

    fn sub(self, other: u32) -> SequenceNumber_t {
        let (new_low, overflow) = self.low.overflowing_sub(other);
        SequenceNumber_t {
            high: match overflow {
                true => self.high - 1,
                false => self.high,
            },
            low: new_low,
        }
    }
}

impl Sub<SequenceNumber_t> for SequenceNumber_t {
    type Output = SequenceNumber_t;

    fn sub(self, other: SequenceNumber_t) -> SequenceNumber_t {
        let (new_low, overflow) = self.low.overflowing_sub(other.low);
        SequenceNumber_t {
            high: match overflow {
                true => self.high - other.high - 1,
                false => self.high - other.high,
            },
            low: new_low,
        }
    }
}

impl PartialOrd for SequenceNumber_t {
    fn partial_cmp(&self, other: &SequenceNumber_t) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SequenceNumber_t {
    fn cmp(&self, other: &SequenceNumber_t) -> Ordering {
        match self.high.cmp(&other.high) {
            Ordering::Equal => self.low.cmp(&other.low),
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
        }
    }
}

#[derive(Debug, PartialEq, Readable, Writable)]
pub struct SequenceNumberSet_t {
    base: SequenceNumber_t,
    set: BitSetRef,
}

#[derive(Debug, PartialEq)]
pub struct BitSetRef(BitSet);

impl BitSetRef {
    pub fn new() -> BitSetRef {
        BitSetRef(BitSet::with_capacity(0))
    }
}

impl Deref for BitSetRef {
    type Target = BitSet;

    fn deref(&self) -> &BitSet {
        &self.0
    }
}

impl DerefMut for BitSetRef {
    fn deref_mut(&mut self) -> &mut BitSet {
        &mut self.0
    }
}

impl<'a, C: Context> Readable<'a, C> for BitSetRef {
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self> {
        let number_of_bits = reader.read_u32()?;
        let mut bit_vec = BitVec::with_capacity(number_of_bits as usize);
        unsafe {
            let mut inner = bit_vec.storage_mut();
            for _ in 0..(number_of_bits / 32) {
                inner.push(reader.read_u32()?);
            }
        }
        Ok(BitSetRef(BitSet::from_bit_vec(bit_vec)))
    }

    #[inline]
    fn minimum_bytes_needed() -> usize {
        4
    }
}

impl<C: Context> Writable<C> for BitSetRef {
    #[inline]
    fn write_to<'a, T: ?Sized + Writer<'a, C>>(&'a self, writer: &mut T) -> Result<()> {
        let bytes = self.0.get_ref().storage();
        writer.write_u32((bytes.len() * 32) as u32)?;
        for byte in bytes {
            writer.write_u32(*byte)?;
        }
        Ok(())
    }
}

impl SequenceNumberSet_t {
    pub fn new(new_base: SequenceNumber_t) -> SequenceNumberSet_t {
        SequenceNumberSet_t {
            base: new_base,
            set: BitSetRef::new(),
        }
    }

    pub fn insert(&mut self, sequence_number: SequenceNumber_t) -> bool {
        if sequence_number >= self.base && sequence_number < self.base + 255 {
            let result = (sequence_number - self.base).value();
            self.set.insert(result as usize);
            return true;
        }
        return false;
    }
}

impl Validity for SequenceNumberSet_t {
    fn valid(&self) -> bool {
        self.base.value() >= 1 && 0 < self.set.len() && self.set.len() <= 256
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    #[test]
    fn sequence_number_set_insert() {
        let mut sequence_number_set =
            SequenceNumberSet_t::new(SequenceNumber_t { high: 0, low: 10 });

        assert!(sequence_number_set.insert(SequenceNumber_t { high: 0, low: 20 }));
        assert!(sequence_number_set.set.contains(20 - 10));

        assert!(!sequence_number_set.insert(SequenceNumber_t { high: 0, low: 5 }));
        assert!(!sequence_number_set.set.contains(5));

        assert!(!sequence_number_set.insert(SequenceNumber_t { high: 1000, low: 7 }));
        assert!(!sequence_number_set.set.contains(7));

        assert!(sequence_number_set.insert(SequenceNumber_t {
            high: 0,
            low: 10 + 200
        }));
        assert_eq!(true, sequence_number_set.set.contains(200));

        assert!(!sequence_number_set.insert(SequenceNumber_t {
            high: 0,
            low: 10 + 255
        }));
        assert_eq!(false, sequence_number_set.set.contains(10 + 255));
    }

    #[test]
    fn sequence_number_starts_by_default_from_one() {
        let default_sequence_number = SequenceNumber_t::default();
        assert_eq!(
            SequenceNumber_t { high: 0, low: 1 },
            default_sequence_number
        );
        assert_eq!(1, default_sequence_number.value());
    }

    #[test]
    fn sequence_number_addition_with_other_sequence_number() {
        {
            let left = SequenceNumber_t { high: 0, low: 0 };
            let right = SequenceNumber_t { high: 0, low: 0 };
            assert_eq!(SequenceNumber_t { high: 0, low: 0 }, left + right);
        }
        {
            let left = SequenceNumber_t { high: 0, low: 20 };
            let right = SequenceNumber_t { high: 0, low: 10 };
            assert_eq!(SequenceNumber_t { high: 0, low: 30 }, left + right);
        }
        {
            let left = SequenceNumber_t { high: 1, low: 20 };
            let right = SequenceNumber_t { high: 0, low: 10 };
            assert_eq!(SequenceNumber_t { high: 1, low: 30 }, left + right);
        }
    }

    #[test]
    fn sequeance_number_addition_with_other_sequence_number_with_low_wrap() {
        let left = SequenceNumber_t {
            high: 0,
            low: <u32>::max_value(),
        };
        let right = SequenceNumber_t { high: 0, low: 1 };
        assert_eq!(SequenceNumber_t { high: 1, low: 0 }, left + right);
    }

    #[test]
    fn sequeance_number_addition_with_other_sequence_number_with_high_wrap() {
        let left = SequenceNumber_t {
            high: <i32>::max_value(),
            low: <u32>::max_value(),
        };
        let right = SequenceNumber_t { high: 0, low: 1 };
        assert!(panic::catch_unwind(|| left + right).is_err());
    }

    #[test]
    fn sequence_number_subtraction_with_other_sequence_number() {
        {
            let left = SequenceNumber_t { high: 0, low: 0 };
            let right = SequenceNumber_t { high: 0, low: 0 };
            assert_eq!(SequenceNumber_t { high: 0, low: 0 }, left - right);
        }
        {
            let left = SequenceNumber_t { high: 0, low: 20 };
            let right = SequenceNumber_t { high: 0, low: 10 };
            assert_eq!(SequenceNumber_t { high: 0, low: 10 }, left - right);
        }
        {
            let left = SequenceNumber_t { high: 1, low: 20 };
            let right = SequenceNumber_t { high: 0, low: 10 };
            assert_eq!(SequenceNumber_t { high: 1, low: 10 }, left - right);
        }
    }

    #[test]
    fn sequeance_number_subtraction_with_other_sequence_number_with_low_wrap() {
        let left = SequenceNumber_t {
            high: 0,
            low: <u32>::min_value(),
        };
        let right = SequenceNumber_t { high: 0, low: 1 };
        assert_eq!(
            SequenceNumber_t {
                high: -1,
                low: <u32>::max_value()
            },
            left - right
        );
    }

    #[test]
    fn sequeance_number_subtraction_with_other_sequence_number_with_high_wrap() {
        let left = SequenceNumber_t {
            high: <i32>::min_value(),
            low: <u32>::min_value(),
        };
        let right = SequenceNumber_t { high: 0, low: 1 };
        assert!(panic::catch_unwind(|| left - right).is_err());
    }

    #[test]
    fn sequeance_number_compare_with_other_sequence_number() {
        assert!(SequenceNumber_t { high: 0, low: 0 } == SequenceNumber_t { high: 0, low: 0 });
        assert!(SequenceNumber_t { high: 0, low: 0 } != SequenceNumber_t { high: 0, low: 1 });
        assert!(SequenceNumber_t { high: 0, low: 0 } != SequenceNumber_t { high: 1, low: 0 });
        assert!(SequenceNumber_t { high: 0, low: 0 } != SequenceNumber_t { high: 1, low: 1 });

        assert!(SequenceNumber_t { high: 0, low: 0 } < SequenceNumber_t { high: 0, low: 1 });
        assert!(SequenceNumber_t { high: 0, low: 0 } < SequenceNumber_t { high: 1, low: 0 });
        assert!(SequenceNumber_t { high: 0, low: 0 } < SequenceNumber_t { high: 1, low: 1 });
        assert!(SequenceNumber_t { high: 0, low: 1 } > SequenceNumber_t { high: 0, low: 0 });
        assert!(SequenceNumber_t { high: 0, low: 1 } == SequenceNumber_t { high: 0, low: 1 });
        assert!(SequenceNumber_t { high: 0, low: 1 } < SequenceNumber_t { high: 1, low: 0 });
        assert!(SequenceNumber_t { high: 0, low: 1 } < SequenceNumber_t { high: 1, low: 1 });

        assert!(SequenceNumber_t { high: 1, low: 0 } > SequenceNumber_t { high: 0, low: 0 });
        assert!(SequenceNumber_t { high: 1, low: 0 } > SequenceNumber_t { high: 0, low: 1 });
        assert!(SequenceNumber_t { high: 1, low: 0 } == SequenceNumber_t { high: 1, low: 0 });
        assert!(SequenceNumber_t { high: 1, low: 0 } < SequenceNumber_t { high: 1, low: 1 });
        assert!(SequenceNumber_t { high: 1, low: 1 } > SequenceNumber_t { high: 0, low: 0 });
        assert!(SequenceNumber_t { high: 1, low: 1 } > SequenceNumber_t { high: 0, low: 1 });
        assert!(SequenceNumber_t { high: 1, low: 1 } > SequenceNumber_t { high: 1, low: 0 });
    }

    serialization_test!( type = SequenceNumber_t,
    {
        sequence_number_default,
        SequenceNumber_t::default(),
        le = [0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
    },
    {
        sequence_number_unknown,
        SEQUENCENUMBER_UNKNOWN,
        le = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00],
        be = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00]
    });

    serialization_test!( type = BitSetRef,
    {
        bit_set_empty,
        BitSetRef::new(),
        le = [0x00, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x00]
    },
    {
        bit_set_non_zero_size,
        (|| {
            let mut set = BitSetRef::new();
            set.insert(0);
            set.insert(42);
            set.insert(7);
            set
        })(),
        le = [0x40, 0x00, 0x00, 0x00,
              0x81, 0x00, 0x00, 0x00,
              0x00, 0x04, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x40,
              0x00, 0x00, 0x00, 0x81,
              0x00, 0x00, 0x04, 0x00]
    });

    serialization_test!( type = SequenceNumberSet_t,
    {
        sequence_number_set_empty,
        SequenceNumberSet_t::new(SequenceNumber_t { high: 7, low: 42 }),
        le = [0x07, 0x00, 0x00, 0x00,  // bitmapBase
              0x2A, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00], // numBits
        be = [0x00, 0x00, 0x00, 0x07,
              0x00, 0x00, 0x00, 0x2A,
              0x00, 0x00, 0x00, 0x00]
    },
    {
        sequence_number_set_manual,
        (|| {
            let mut set = SequenceNumberSet_t::new(SequenceNumber_t { high: 1145324612, low: 268435456 });
            set.insert(SequenceNumber_t { high: 1145324612, low: 268435457 });
            set.insert(SequenceNumber_t { high: 1145324612, low: 268435459 });
            set.insert(SequenceNumber_t { high: 1145324612, low: 268435460 });
            set.insert(SequenceNumber_t { high: 1145324612, low: 268435462 });
            set.insert(SequenceNumber_t { high: 1145324612, low: 268435464 });
            set.insert(SequenceNumber_t { high: 1145324612, low: 268435466 });
            set.insert(SequenceNumber_t { high: 1145324612, low: 268435469 });
            set
        })(),
        le = [0x44, 0x44, 0x44, 0x44,
              0x00, 0x00, 0x00, 0x10,
              0x20, 0x00, 0x00, 0x00,
              0x5A, 0x25, 0x00, 0x00],
        be = [0x44, 0x44, 0x44, 0x44,
              0x10, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x20,
              0x00, 0x00, 0x25, 0x5A]
    });
}
