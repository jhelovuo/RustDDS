use speedy::{Readable, Writable};
use std::convert::From;

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Writable)]
pub struct Count_t {
    value: i32,
}

impl From<i32> for Count_t {
    fn from(value: i32) -> Self {
        Count_t { value }
    }
}

impl From<Count_t> for i32 {
    fn from(count: Count_t) -> Self {
        count.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = Count_t,
        {
            count_test_one,
            Count_t::from(1),
            le = [0x01, 0x00, 0x00, 0x00],
            be = [0x00, 0x00, 0x00, 0x01]
        },
        {
            count_test_min,
            Count_t::from(0),
            le = [0x00, 0x00, 0x00, 0x00],
            be = [0x00, 0x00, 0x00, 0x00]
        },
        {
            count_test_high,
            Count_t::from(0x3BCDEF01),
            le = [0x01, 0xEF, 0xCD, 0x3B],
            be = [0x3B, 0xCD, 0xEF, 0x01]
        },
        {
            count_test_random,
            Count_t::from(0x1EADBEFF),
            le = [0xFF, 0xBE, 0xAD, 0x1E],
            be = [0x1E, 0xAD, 0xBE, 0xFF]
        }
    );
}
