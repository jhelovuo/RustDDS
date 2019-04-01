use crate::common::ranged_bit_set::RangedBitSet;
use crate::messages::fragment_number::FragmentNumber_t;
use speedy_derive::{Readable, Writable};
use std::convert::TryFrom;
use std::ops::Sub;

pub type FragmentNumberSet_t = RangedBitSet<FragmentNumber_t>;

impl Sub for FragmentNumber_t {
    type Output = FragmentNumber_t;

    fn sub(self, other: FragmentNumber_t) -> Self::Output {
        FragmentNumber_t::from(u32::from(self) - u32::from(other))
    }
}

impl TryFrom<FragmentNumber_t> for u8 {
    type Error = std::num::TryFromIntError;

    fn try_from(value: FragmentNumber_t) -> Result<u8, Self::Error> {
        u8::try_from(u32::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = FragmentNumberSet_t,
    {
        fragment_number_set_empty,
        FragmentNumberSet_t::new(FragmentNumber_t::from(42)),
        le = [0x2A, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x2A,
              0x00, 0x00, 0x00, 0x00]
    },
    {
        fragment_number_set_manual,
        (|| {
            let mut set = FragmentNumberSet_t::new(FragmentNumber_t::from(1000));
            set.insert(FragmentNumber_t::from(1001));
            set.insert(FragmentNumber_t::from(1003));
            set.insert(FragmentNumber_t::from(1004));
            set.insert(FragmentNumber_t::from(1006));
            set.insert(FragmentNumber_t::from(1008));
            set.insert(FragmentNumber_t::from(1010));
            set.insert(FragmentNumber_t::from(1013));
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
