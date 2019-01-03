use std::cmp::Ordering;
use std::ops::Add;
use std::ops::Sub;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Readable, Writable)]
pub struct SequenceNumber_t {
    pub high: i32,
    pub low: u32,
}

impl SequenceNumber_t {
    pub const SEQUENCENUMBER_UNKNOWN: SequenceNumber_t = SequenceNumber_t { high: -1, low: 0 };
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

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
        SequenceNumber_t::SEQUENCENUMBER_UNKNOWN,
        le = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00],
        be = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00]
    });
}
