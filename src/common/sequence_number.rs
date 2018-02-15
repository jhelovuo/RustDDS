use std::ops::Add;
use std::ops::Sub;
use std::cmp::Ordering;
use bit_set::BitSet;
use message::validity_trait::Validity;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
        SequenceNumber_t {
            high: self.high + other.high,
            low: self.low + other.low
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
        SequenceNumber_t {
            high: self.high - other.high,
            low: self.low - other.low
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

pub struct SequenceNumberSet_t {
    base: SequenceNumber_t,
    num_bits: u32,
    set: BitSet
}

impl SequenceNumberSet_t {
    pub fn new(new_base: SequenceNumber_t, number_of_bits: usize) -> SequenceNumberSet_t {
        SequenceNumberSet_t {
            base: new_base,
            num_bits: number_of_bits as u32,
            set: BitSet::with_capacity(number_of_bits)
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
        unimplemented!(); // TODO: finish me
    }
}

#[test]
fn sequence_number_addition_with_other_sequence_number() {
    {
        let left = SequenceNumber_t { high: 0, low: 0 };
        let right = SequenceNumber_t { high: 0, low: 0 };
        assert_eq!(SequenceNumber_t { high: 0, low: 0 }, left+right);
    }
    {
        let left = SequenceNumber_t { high: 0, low: 20 };
        let right = SequenceNumber_t { high: 0, low: 10 };
        assert_eq!(SequenceNumber_t { high: 0, low: 30 }, left+right);
    }
    {
        let left = SequenceNumber_t { high: 1, low: 20 };
        let right = SequenceNumber_t { high: 0, low: 10 };
        assert_eq!(SequenceNumber_t { high: 1, low: 30 }, left+right);
    }
}

#[test]
#[should_panic]
fn sequeance_number_addition_with_other_sequence_number_with_wrap() {
    let left = SequenceNumber_t { high: <i32>::max_value(), low: <u32>::max_value() };
    let right = SequenceNumber_t { high: 0, low: 1 };
    left + right;
}

#[test]
fn sequence_number_subtraction_with_other_sequence_number() {
    {
        let left = SequenceNumber_t { high: 0, low: 0 };
        let right = SequenceNumber_t { high: 0, low: 0 };
        assert_eq!(SequenceNumber_t { high: 0, low: 0 }, left-right);
    }
    {
        let left = SequenceNumber_t { high: 0, low: 20 };
        let right = SequenceNumber_t { high: 0, low: 10 };
        assert_eq!(SequenceNumber_t { high: 0, low: 10 }, left-right);
    }
    {
        let left = SequenceNumber_t { high: 1, low: 20 };
        let right = SequenceNumber_t { high: 0, low: 10 };
        assert_eq!(SequenceNumber_t { high: 1, low: 10 }, left-right);
    }
}

#[test]
#[should_panic]
fn sequeance_number_subtraction_with_other_sequence_number_with_wrap() {
    let left = SequenceNumber_t { high: 0, low: 0 };
    let right = SequenceNumber_t { high: 0, low: 10 };
    left - right;
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

#[test]
fn sequence_number_set_insert() {
    let mut sequence_number_set = SequenceNumberSet_t::new(SequenceNumber_t{
        high: 0,
        low: 10
    }, 32);

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
