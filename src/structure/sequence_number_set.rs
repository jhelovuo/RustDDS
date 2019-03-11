use crate::common::bit_set::BitSetRef;
use crate::common::validity_trait::Validity;
use crate::structure::sequence_number::SequenceNumber_t;
use speedy_derive::{Readable, Writable};

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
        if sequence_number >= self.base && i64::from(sequence_number) < i64::from(self.base) + 255 {
            self.set
                .insert((i64::from(sequence_number) - i64::from(self.base)) as usize);
            return true;
        }
        return false;
    }
}

impl Validity for SequenceNumberSet_t {
    fn valid(&self) -> bool {
        i64::from(self.base) >= 1 && 0 < self.set.len() && self.set.len() <= 256
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_number_set_insert() {
        let mut sequence_number_set = SequenceNumberSet_t::new(SequenceNumber_t::from(10));

        assert!(sequence_number_set.insert(SequenceNumber_t::from(20)));
        assert!(sequence_number_set.set.contains(20 - 10));

        assert!(!sequence_number_set.insert(SequenceNumber_t::from(5)));
        assert!(!sequence_number_set.set.contains(5));

        assert!(!sequence_number_set.insert(SequenceNumber_t::from(10000)));
        assert!(!sequence_number_set.set.contains(7));

        assert!(sequence_number_set.insert(SequenceNumber_t::from(10 + 200)));
        assert_eq!(true, sequence_number_set.set.contains(200));

        assert!(!sequence_number_set.insert(SequenceNumber_t::from(10 + 255)));
        assert_eq!(false, sequence_number_set.set.contains(10 + 255));
    }

    serialization_test!( type = SequenceNumberSet_t,
    {
        sequence_number_set_empty,
        SequenceNumberSet_t::new(SequenceNumber_t::from(42)),
        le = [0x00, 0x00, 0x00, 0x00,  // bitmapBase
              0x2A, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00], // numBits
        be = [0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x2A,
              0x00, 0x00, 0x00, 0x00]
    },
    {
        sequence_number_set_manual,
        (|| {
            let mut set = SequenceNumberSet_t::new(SequenceNumber_t::from(1000));
            set.insert(SequenceNumber_t::from(1001));
            set.insert(SequenceNumber_t::from(1003));
            set.insert(SequenceNumber_t::from(1004));
            set.insert(SequenceNumber_t::from(1006));
            set.insert(SequenceNumber_t::from(1008));
            set.insert(SequenceNumber_t::from(1010));
            set.insert(SequenceNumber_t::from(1013));
            set
        })(),
        le = [0x00, 0x00, 0x00, 0x00,
              0xE8, 0x03, 0x00, 0x00,
              0x20, 0x00, 0x00, 0x00,
              0x5A, 0x25, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x03, 0xE8,
              0x00, 0x00, 0x00, 0x20,
              0x00, 0x00, 0x25, 0x5A]
    });
}
