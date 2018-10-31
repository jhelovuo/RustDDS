extern crate time;

use std::convert::{From, Into};
use std::cmp::Ordering;
use self::time::Timespec;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Time_t {
    pub seconds: i32,
    pub fraction: u32
}

pub type Timestamp = Time_t;

pub const TIME_ZERO: Time_t = Time_t { seconds: 0, fraction: 0 };
pub const TIME_INVALID: Time_t = Time_t { seconds: -1, fraction: 0xFFFFFFFF };
pub const TIME_INFINITE: Time_t = Time_t { seconds: 0x7FFFFFFF, fraction: 0xFFFFFFFF };

impl Time_t {
    pub fn value(&self) -> i64 {
        self.seconds as i64 + ((self.fraction as i64) << 32)
    }
}

impl From<time::Timespec> for Time_t {
    fn from(timespec: time::Timespec) -> Self {
        Time_t {
            seconds: timespec.sec as i32,
            fraction: timespec.nsec as u32
        }
    }
}

impl Into<time::Timespec> for Time_t {
    fn into(self) -> time::Timespec {
        time::Timespec {
            sec: self.seconds as i64,
            nsec: self.fraction as i32
        }
    }
}

impl PartialOrd for Time_t {
    fn partial_cmp(&self, other: &Time_t) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time_t {
    fn cmp(&self, other: &Time_t) -> Ordering {
        match self.seconds.cmp(&other.seconds) {
            Ordering::Equal => self.fraction.cmp(&other.fraction),
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    assert_ser_de!({
        time_zero,
        TIME_ZERO,
        le = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    });

    assert_ser_de!({
        time_invalid,
        TIME_INVALID,
        le = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        be = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    });

    assert_ser_de!({
        time_infinite,
        TIME_INFINITE,
        le = [0xFF, 0xFF, 0xFF, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF],
        be = [0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    });

    assert_ser_de!({
        time_current_empty_fraction,
        Time_t { seconds: 1537045491, fraction: 0 },
        le = [0xF3, 0x73, 0x9D, 0x5B, 0x00, 0x00, 0x00, 0x00],
        be = [0x5B, 0x9D, 0x73, 0xF3, 0x00, 0x00, 0x00, 0x00]
    });

    assert_ser_de!({
        time_from_wireshark,
        Time_t { seconds: 1519152760, fraction: 1328210046 },
        le = [0x78, 0x6E, 0x8C, 0x5A, 0x7E, 0xE0, 0x2A, 0x4F],
        be = [0x5A, 0x8C, 0x6E, 0x78, 0x4F, 0x2A, 0xE0, 0x7E]
    });

    #[test]
    fn convert_from_timespec() {
        let timespec = time::Timespec { sec: 1519152760, nsec: 1328210046 };
        let time: Time_t = timespec.into();

        assert_eq!(time, Time_t { seconds: 1519152760, fraction: 1328210046 });
    }

    #[test]
    fn convert_to_timespec() {
        let time = Time_t { seconds: 1519152760, fraction: 1328210046 };
        let timespec: time::Timespec = time.into();

        assert_eq!(timespec, time::Timespec { sec: 1519152760, nsec: 1328210046 });
    }
}
