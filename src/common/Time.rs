#[derive(PartialOrd, PartialEq, Ord, Eq)]
struct Time_t {
    seconds: i64,
    fraction: u64
}

const TIME_ZERO: Time_t = Time_t { seconds: 0, fraction: 0 };
const TIME_INVALID: Time_t = Time_t { seconds: -1, fraction: 0xFFFFFFFF };
const TIME_INFINITE: Time_t = Time_t { seconds: 0x7FFFFFFF, fraction: 0xFFFFFFFF };

impl Time_t {
    fn value(&self) -> i64 {
        self.seconds + ((self.fraction as i64) << 32)
    }
}
