#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct SequenceNumber_t {
    pub high: i64,
    pub low: u64
}

pub const SEQUENCENUMBER_UNKNOWN: SequenceNumber_t = SequenceNumber_t { high: -1, low: 0 };

impl SequenceNumber_t {
    pub fn value(&self) -> u64 {
        ((self.high as u64) << 32) + self.low
    }
}
