extern crate time;

use self::time::Timespec;

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Ord, Eq)]
pub struct Time_t {
    seconds: i32,
    fraction: u32
}

pub const TIME_ZERO: Time_t = Time_t { seconds: 0, fraction: 0 };
pub const TIME_INVALID: Time_t = Time_t { seconds: -1, fraction: 0xFFFFFFFF };
pub const TIME_INFINITE: Time_t = Time_t { seconds: 0x7FFFFFFF, fraction: 0xFFFFFFFF };

impl Time_t {
    pub fn new(timespec: time::Timespec) -> Time_t {
        Time_t {
            seconds: timespec.sec as i32,
            fraction: timespec.nsec as u32
        }
    }

    pub fn value(&self) -> i64 {
        self.seconds as i64 + ((self.fraction as i64) << 32)
    }

    pub fn to_timespec(&self) -> time::Timespec {
        time::Timespec {
            sec: self.seconds as i64,
            nsec: self.fraction as i32
        }
    }
}
