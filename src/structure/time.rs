extern crate time;

use speedy::{Readable, Writable};
use std::convert::From;

/// The representation of the time is the one defined by the IETF Network Time
/// Protocol (NTP) Standard (IETF RFC 1305). In this representation, time is
/// expressed in seconds and fraction of seconds using the formula:
/// time = seconds + (fraction / 2^(32))

/// This time representation is used in RTPS messages.
/// Application-facing interfaces should use Instant and Duration from Rust std library.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Readable, Writable, Clone, Copy)]
pub struct Time {
  seconds: i32,
  fraction: u32,
}

pub type Timestamp = Time;

impl Time {
  pub const TIME_ZERO: Time = Time {
    seconds: 0,
    fraction: 0,
  };
  pub const TIME_INVALID: Time = Time {
    seconds: -1,
    fraction: 0xFFFF_FFFF,
  };
  pub const TIME_INFINITE: Time = Time {
    seconds: 0x7FFF_FFFF,
    fraction: 0xFFFF_FFFF,
  };
}

const NANOS_PER_SEC: i64 = 1_000_000_000;

impl From<time::Timespec> for Time {
  fn from(timespec: time::Timespec) -> Self {
    Time {
      seconds: timespec.sec as i32,
      fraction: ((i64::from(timespec.nsec) << 32) / NANOS_PER_SEC) as u32,
    }
  }
}

impl From<Time> for time::Timespec {
  fn from(time: Time) -> Self {
    time::Timespec {
      sec: i64::from(time.seconds),
      nsec: ((i64::from(time.fraction) * NANOS_PER_SEC) >> 32) as i32,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = Time,
  {
      time_zero,
      Time::TIME_ZERO,
      le = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
  },
  {
      time_invalid,
      Time::TIME_INVALID,
      le = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
      be = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  },
  {
      time_infinite,
      Time::TIME_INFINITE,
      le = [0xFF, 0xFF, 0xFF, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF],
      be = [0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  },
  {
      time_current_empty_fraction,
      Time { seconds: 1_537_045_491, fraction: 0 },
      le = [0xF3, 0x73, 0x9D, 0x5B, 0x00, 0x00, 0x00, 0x00],
      be = [0x5B, 0x9D, 0x73, 0xF3, 0x00, 0x00, 0x00, 0x00]
  },
  {
      time_from_wireshark,
      Time { seconds: 1_519_152_760, fraction: 1_328_210_046 },
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
                    let time = Time::from($timespec);
                    let epsilon = (FRACS_PER_SEC / NANOS_PER_SEC) as u32 + 1;

                    assert_eq!($time.seconds, time.seconds);
                    assert_ge_at_most_by!(epsilon, $time.fraction, time.fraction);
                }

                #[test]
                fn time_from_eq_into_time() {
                    let time_from = Time::from($timespec);
                    let into_time: Time = $timespec.into();

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
      time = Time::TIME_ZERO,
      timespec = time::Timespec {
          sec: 0,
          nsec: 0,
      },
  },
  {
      convert_time_non_zero,
      time = Time {
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
      time = Time::TIME_INVALID,
      timespec = time::Timespec {
          sec: -1,
          nsec: 999_999_999,
      },
  },
  {
      convert_time_infinite,
      time = Time::TIME_INFINITE,
      timespec = time::Timespec {
          sec: 0x7FFFFFFF,
          nsec: 999_999_999,
      },
  },
  {
      convert_time_non_infinite,
      time = Time {
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
      time = Time {
          seconds: 0x40000000,
          fraction: 0x80000000,
      },
      timespec = time::Timespec {
          sec: 0x40000000,
          nsec: 500_000_000,
      },
  });
}
