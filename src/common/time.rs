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
