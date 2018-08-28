use std::ops::Add;
use std::ops::Sub;
use std::ops::{Deref, DerefMut};
use std::cmp::Ordering;
use bit_set::BitSet;
use bit_vec::BitVec;
use message::validity_trait::Validity;
use serde::ser::{Serialize, Serializer, SerializeStruct, SerializeSeq};
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess};
use std::{cmp, fmt};
use std::convert::{From, Into};
use std::marker::PhantomData;

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct SequenceNumber_t {
    pub high: i32,
    pub low: u32
}

pub const SEQUENCENUMBER_UNKNOWN: SequenceNumber_t = SequenceNumber_t { high: -1, low: 0 };

impl SequenceNumber_t {
    pub fn value(&self) -> u64 {
        ((self.high as u64) << 32) + self.low as u64
    }
}

impl Default for SequenceNumber_t {
    fn default() -> SequenceNumber_t {
        SequenceNumber_t {
            high: 0,
            low: 1
        }
    }
}

impl Add<u32> for SequenceNumber_t {
    type Output = SequenceNumber_t;

    fn add(self, other: u32) -> SequenceNumber_t {
        let (new_low, overflow) = self.low.overflowing_add(other);

        SequenceNumber_t {
            high: match overflow {
                true => self.high + 1,
                false => self.high
            },
            low: new_low
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
                false => self.high + other.high
            },
            low: new_low
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
                false => self.high
            },
            low: new_low
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
                false => self.high - other.high
            },
            low: new_low
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
            Ordering::Greater => Ordering::Greater
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SequenceNumberSet_t {
    base: SequenceNumber_t,
    set: BitSetRef
}

struct BitSetRef(BitSet);

impl From<BitSet> for BitSetRef {
    fn from(bit_set: BitSet) -> Self {
        BitSetRef(bit_set)
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

impl Serialize for BitSetRef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.capacity()))?;
        for byte in self.iter()
        {
            sequence.serialize_element(&byte)?;
        }
        sequence.end()
    }
}

impl<'de> Deserialize<'de> for BitSetRef {
    fn deserialize<D>(deserializer: D) -> Result<BitSetRef, D::Error>
        where D: Deserializer<'de>
{
        struct BitSetRefVisitor;

        impl<'de> Visitor<'de> for BitSetRefVisitor {
            type Value = BitSetRef;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a nonempty sequence of numbers")
            }

            fn visit_seq<S>(self, mut seq: S) -> Result<BitSetRef, S::Error>
                where S: SeqAccess<'de>
            {
                let mut bit_set: BitSetRef = BitSet::with_capacity(seq.size_hint().unwrap_or(0)).into();

                while let Some(value) = seq.next_element()? {
                    bit_set.insert(value);
                }

                Ok(bit_set)
            }
        }

        deserializer.deserialize_seq(BitSetRefVisitor)
    }
}

impl SequenceNumberSet_t {
    pub fn new(new_base: SequenceNumber_t) -> SequenceNumberSet_t {
        SequenceNumberSet_t {
            base: new_base,
            set: BitSet::with_capacity(256).into()
        }
    }

    pub fn insert(&mut self, sequence_number: SequenceNumber_t) -> bool {
        if (sequence_number >= self.base && sequence_number < self.base + 255)
        {
            let result = (sequence_number - self.base).value();
            self.set.insert(result as usize);
            return true;
        }
        return false;
    }
}

impl Validity for SequenceNumberSet_t {
    fn valid(&self) -> bool {
        self.base.value() >= 1 &&
            0 < self.set.capacity() &&
            self.set.capacity() <= 256
    }
}

#[test]
fn sequence_number_set_insert() {
    let mut sequence_number_set = SequenceNumberSet_t::new(
        SequenceNumber_t{
            high: 0,
            low: 10
    });

    assert!(sequence_number_set.insert(SequenceNumber_t{ high: 0, low: 20 }));
    assert!(sequence_number_set.set.contains(20-10));

    assert!(!sequence_number_set.insert(SequenceNumber_t{ high: 0, low: 5 }));
    assert!(!sequence_number_set.set.contains(5));

    assert!(!sequence_number_set.insert(SequenceNumber_t{ high: 1000, low: 7 }));
    assert!(!sequence_number_set.set.contains(7));

    assert!(sequence_number_set.insert(SequenceNumber_t{ high: 0, low: 10 + 200 }));
    assert_eq!(true, sequence_number_set.set.contains(200));

    assert!(!sequence_number_set.insert(SequenceNumber_t{ high: 0, low: 10 + 255 }));
    assert_eq!(false, sequence_number_set.set.contains(10 + 255));
}
