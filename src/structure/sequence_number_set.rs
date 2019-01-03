use crate::common::bit_set::BitSetRef;
use crate::common::validity_trait::Validity;
use crate::structure::sequence_number::SequenceNumber_t;

#[derive(Debug, PartialEq, Readable, Writable)]
pub struct SequenceNumberSet_t {
    base: SequenceNumber_t,
    set: BitSetRef,
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
            let mut set = SequenceNumberSet_t::new(SequenceNumber_t {
                high: 1145324612,
                low: 268435456
            });
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
