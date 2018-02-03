#[derive(Serialize, Deserialize, PartialOrd, PartialEq, Ord, Eq)]
pub struct Time_t {
    seconds: i32,
    fraction: u32
}

pub const TIME_ZERO: Time_t = Time_t { seconds: 0, fraction: 0 };
pub const TIME_INVALID: Time_t = Time_t { seconds: -1, fraction: 0xFFFFFFFF };
pub const TIME_INFINITE: Time_t = Time_t { seconds: 0x7FFFFFFF, fraction: 0xFFFFFFFF };

impl Time_t {
    pub fn value(&self) -> i64 {
        self.seconds as i64 + ((self.fraction as i64) << 32)
    }
}
