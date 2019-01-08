use crate::common::bit_set::BitSetRef;
use crate::common::validity_trait::Validity;
use crate::messages::fragment_number::FragmentNumber_t;
use speedy_derive::{Readable, Writable};

#[derive(Debug, PartialEq, Readable, Writable)]
pub struct FragmentNumberSet_t {
    base: FragmentNumber_t,
    set: BitSetRef,
}

impl FragmentNumberSet_t {
    pub fn new(new_base: FragmentNumber_t) -> FragmentNumberSet_t {
        FragmentNumberSet_t {
            base: new_base,
            set: BitSetRef::new(),
        }
    }

    pub fn insert(&mut self, fragment_number: FragmentNumber_t) -> bool {
        match self.is_in_range(fragment_number) {
            true => {
                let offset = self.base_offset(fragment_number);
                self.set.insert(offset)
            }
            false => false,
        }
    }

    pub fn contains(&self, fragment_number: FragmentNumber_t) -> bool {
        match self.is_in_range(fragment_number) {
            true => self.set.contains(self.base_offset(fragment_number)),
            false => false,
        }
    }

    fn is_in_range(&self, fragment_number: FragmentNumber_t) -> bool {
        fragment_number >= self.base && fragment_number.value <= self.base.value + 255
    }

    fn base_offset(&self, fragment_number: FragmentNumber_t) -> usize {
        (fragment_number.value - self.base.value) as usize
    }
}

impl Validity for FragmentNumberSet_t {
    fn valid(&self) -> bool {
        self.base.value >= 1 && 0 < self.set.len() && self.set.len() <= 256
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_number_set_insert() {
        let base = 10;
        let mut fragment_number_set = FragmentNumberSet_t::new(FragmentNumber_t { value: base });

        assert_eq!(
            false,
            fragment_number_set.contains(FragmentNumber_t { value: base })
        );

        assert!(fragment_number_set.insert(FragmentNumber_t { value: base }));
        assert!(fragment_number_set.contains(FragmentNumber_t { value: base }));

        let max_range = base + 255;
        assert!(fragment_number_set.insert(FragmentNumber_t { value: max_range }));
        assert!(fragment_number_set.contains(FragmentNumber_t { value: max_range }));

        let below_range = base - 1;
        assert_eq!(
            false,
            fragment_number_set.insert(FragmentNumber_t { value: below_range })
        );
        assert_eq!(
            false,
            fragment_number_set.contains(FragmentNumber_t { value: below_range })
        );

        let above_max_range = max_range + 1;
        assert_eq!(
            false,
            fragment_number_set.insert(FragmentNumber_t {
                value: above_max_range
            })
        );
        assert_eq!(
            false,
            fragment_number_set.contains(FragmentNumber_t {
                value: above_max_range
            })
        );

        assert_eq!(
            false,
            fragment_number_set.insert(FragmentNumber_t { value: base })
        );
    }

    serialization_test!( type = FragmentNumberSet_t,
    {
        fragment_number_set_empty,
        FragmentNumberSet_t::new(FragmentNumber_t { value: 42 }),
        le = [0x2A, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x2A,
              0x00, 0x00, 0x00, 0x00]
    },
    {
        fragment_number_set_manual,
        (|| {
            let mut set = FragmentNumberSet_t::new(FragmentNumber_t {
                value: 1000
            });
            set.insert(FragmentNumber_t {  value: 1001 });
            set.insert(FragmentNumber_t {  value: 1003 });
            set.insert(FragmentNumber_t {  value: 1004 });
            set.insert(FragmentNumber_t {  value: 1006 });
            set.insert(FragmentNumber_t {  value: 1008 });
            set.insert(FragmentNumber_t {  value: 1010 });
            set.insert(FragmentNumber_t {  value: 1013 });
            set
        })(),
        le = [0xE8, 0x03, 0x00, 0x00,
              0x20, 0x00, 0x00, 0x00,
              0x5A, 0x25, 0x00, 0x00],
        be = [0x00, 0x00, 0x03, 0xE8,
              0x00, 0x00, 0x00, 0x20,
              0x00, 0x00, 0x25, 0x5A]
    });
}
