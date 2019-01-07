extern crate time;

use std::cmp::Ordering;
use std::convert::From;

/// The representation of the time is the one defined by the IETF Network Time Protocol (NTP)
/// Standard (IETF RFC 1305). In this representation, time is expressed in seconds and fraction
/// of seconds using the formula:
/// time = seconds + (fraction / 2^(32))
#[derive(Debug, PartialEq, Eq, Readable, Writable)]
pub struct Time_t {
    pub seconds: i32,
    pub fraction: u32,
}

pub type Timestamp = Time_t;

impl Time_t {
    pub const TIME_ZERO: Time_t = Time_t {
        seconds: 0,
        fraction: 0,
    };
    pub const TIME_INVALID: Time_t = Time_t {
        seconds: -1,
        fraction: 0xFFFFFFFF,
    };
    pub const TIME_INFINITE: Time_t = Time_t {
        seconds: 0x7FFFFFFF,
        fraction: 0xFFFFFFFF,
    };
}

const NANOS_PER_SEC: i64 = 1_000_000_000;

impl From<time::Timespec> for Time_t {
    fn from(timespec: time::Timespec) -> Self {
        Time_t {
            seconds: timespec.sec as i32,
            fraction: (((timespec.nsec as i64) << 32) / NANOS_PER_SEC) as u32,
        }
    }
}

impl From<Time_t> for time::Timespec {
    fn from(time: Time_t) -> Self {
        time::Timespec {
            sec: time.seconds as i64,
            nsec: (((time.fraction as i64) * NANOS_PER_SEC) >> 32) as i32,
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
            Ordering::Greater => Ordering::Greater,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = Time_t,
    {
        time_zero,
        Time_t::TIME_ZERO,
        le = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    },
    {
        time_invalid,
        Time_t::TIME_INVALID,
        le = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        be = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    },
    {
        time_infinite,
        Time_t::TIME_INFINITE,
        le = [0xFF, 0xFF, 0xFF, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF],
        be = [0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    },
    {
        time_current_empty_fraction,
        Time_t { seconds: 1_537_045_491, fraction: 0 },
        le = [0xF3, 0x73, 0x9D, 0x5B, 0x00, 0x00, 0x00, 0x00],
        be = [0x5B, 0x9D, 0x73, 0xF3, 0x00, 0x00, 0x00, 0x00]
    },
    {
        time_from_wireshark,
        Time_t { seconds: 1_519_152_760, fraction: 1_328_210_046 },
        le = [0x78, 0x6E, 0x8C, 0x5A, 0x7E, 0xE0, 0x2A, 0x4F],
        be = [0x5A, 0x8C, 0x6E, 0x78, 0x4F, 0x2A, 0xE0, 0x7E]
    });

    macro_rules! conversion_test {
        ($({ $name:ident, time = $time:expr, timespec = $timespec:expr, }),+) => {
            $(mod $name {
                use super::*;

                const FRACS_PER_SEC: i64 = 0x100000000;

                macro_rules! assert_ge_at_most_by {
                    ($e:expr, $x:expr, $y:expr) => {
                        assert!($x >= $y);
                        assert!($x - $y <= $e);
                    }
                }

                #[test]
                fn time_from_timespec() {
                    let time = Time_t::from($timespec);
                    let epsilon = (FRACS_PER_SEC / NANOS_PER_SEC) as u32 + 1;

                    assert_eq!($time.seconds, time.seconds);
                    assert_ge_at_most_by!(epsilon, $time.fraction, time.fraction);
                }

                #[test]
                fn time_from_eq_into_time() {
                    let time_from = Time_t::from($timespec);
                    let into_time: Time_t = $timespec.into();

                    assert_eq!(time_from, into_time);
                }

                #[test]
                fn timespec_from_time() {
                    let timespec = time::Timespec::from($time);
                    let epsilon = (NANOS_PER_SEC / FRACS_PER_SEC) as i32 + 1;

                    assert_eq!($timespec.sec, timespec.sec);
                    assert_ge_at_most_by!(epsilon, $timespec.nsec, timespec.nsec);
                }

                #[test]
                fn timespec_from_eq_into_timespec() {
                    let timespec_from = time::Timespec::from($time);
                    let into_timespec: time::Timespec = $time.into();

                    assert_eq!(timespec_from, into_timespec);
                }
            })+
        }
    }

    conversion_test!(
    {
        convert_time_zero,
        time = Time_t::TIME_ZERO,
        timespec = time::Timespec {
            sec: 0,
            nsec: 0,
        },
    },
    {
        convert_time_non_zero,
        time = Time_t {
            seconds: 1,
            fraction: 5,
        },
        timespec = time::Timespec {
            sec: 1,
            nsec: 1,
        },
    },
    {
        convert_time_invalid,
        time = Time_t::TIME_INVALID,
        timespec = time::Timespec {
            sec: -1,
            nsec: 999_999_999,
        },
    },
    {
        convert_time_infinite,
        time = Time_t::TIME_INFINITE,
        timespec = time::Timespec {
            sec: 0x7FFFFFFF,
            nsec: 999_999_999,
        },
    },
    {
        convert_time_non_infinite,
        time = Time_t {
            seconds: 0x7FFFFFFF,
            fraction: 0xFFFFFFFA,
        },
        timespec = time::Timespec {
            sec: 0x7FFFFFFF,
            nsec: 999_999_998,
        },
    },
    {
        convert_time_half_range,
        time = Time_t {
            seconds: 0x40000000,
            fraction: 0x80000000,
        },
        timespec = time::Timespec {
            sec: 0x40000000,
            nsec: 500_000_000,
        },
    });
}
