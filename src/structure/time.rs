use speedy::{Readable, Writable};
use serde::{Serialize, Deserialize};
use std::ops::Sub;
use chrono;

use super::duration::Duration;

/// Quoting RTPS 2.3 spec 9.3.2.1:
///
/// > The representation of the time is the one defined by the IETF Network Time
/// > Protocol (NTP) Standard (IETF RFC 1305). In this representation, time is
/// > expressed in seconds and fraction of seconds using the formula:
/// > time = seconds + (fraction / 2^(32))
///
/// > The time origin is represented by the reserved value TIME_ZERO and corresponds
/// > to the UNIX prime epoch 0h, 1 January 1970.
///
///
/// *Note* : NTP does not use the Unix epoch (1970-01-01 00:00) but the beginning of
/// the 20th century epoch (1900-01-01 00:00) insted. So these timestamps are not the same
/// as in NTP.

/// This time representation is used in RTPS messages.
/// This is called Time_t in the RTPS spec.
#[derive(
  Debug, PartialEq, Eq, PartialOrd, Ord, Readable, Writable, Clone, Copy, Serialize, Deserialize,
)]
pub struct Timestamp {
  seconds: u32,
  fraction: u32,
}

impl Timestamp {
  // Special contants reserved by the RTPS protocol, from RTPS spec section 9.3.2.
  pub const TIME_ZERO: Timestamp = Timestamp {
    seconds: 0,
    fraction: 0,
  };
  pub const TIME_INVALID: Timestamp = Timestamp {
    seconds: 0xFFFF_FFFF,
    fraction: 0xFFFF_FFFF,
  };
  pub const TIME_INFINITE: Timestamp = Timestamp {
    seconds: 0x7FFF_FFFF,
    fraction: 0xFFFF_FFFF,
  };

  pub fn now() -> Timestamp {
    Timestamp::from_nanos(chrono::Utc::now().timestamp_nanos() as u64)
  }

  fn to_ticks(&self) -> u64 {
    ((self.seconds as u64) << 32) + (self.fraction as u64)
  }

  fn from_ticks(ticks: u64) -> Timestamp {
    Timestamp {
      seconds: (ticks >> 32) as u32,
      fraction: ticks as u32,
    }
  }

  fn from_nanos(nanos_since_unix_epoch: u64) -> Timestamp {
    Timestamp {
      seconds: (nanos_since_unix_epoch / 1_000_000_000) as u32,
      fraction: (((nanos_since_unix_epoch % 1_000_000_000) << 32) / 1_000_000_000) as u32,
    }
  }

  pub fn duration_since(&self, since: Timestamp) -> Duration {
    *self - since
  }
}

impl Sub for Timestamp {
  type Output = Duration;

  fn sub(self, other: Timestamp) -> Duration {
    let a = self.to_ticks();
    let b = other.to_ticks();
    // https://doc.rust-lang.org/1.30.0/book/first-edition/casting-between-types.html
    // "Casting between two integers of the same size (e.g. i32 -> u32) is a no-op"
    Duration::from_ticks(a.wrapping_sub(b) as i64)
  }
}

impl Sub<Duration> for Timestamp {
  type Output = Timestamp;

  fn sub(self, rhs: Duration) -> Self::Output {
    let lhs_ticks = self.to_ticks();
    let rhs_ticks = rhs.to_ticks() as u64;

    Timestamp::from_ticks(lhs_ticks - rhs_ticks)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = Timestamp,
  {
      time_zero,
      Timestamp::TIME_ZERO,
      le = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
  },
  {
      time_invalid,
      Timestamp::TIME_INVALID,
      le = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
      be = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  },
  {
      time_infinite,
      Timestamp::TIME_INFINITE,
      le = [0xFF, 0xFF, 0xFF, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF],
      be = [0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  },
  {
      time_current_empty_fraction,
      Timestamp { seconds: 1_537_045_491, fraction: 0 },
      le = [0xF3, 0x73, 0x9D, 0x5B, 0x00, 0x00, 0x00, 0x00],
      be = [0x5B, 0x9D, 0x73, 0xF3, 0x00, 0x00, 0x00, 0x00]
  },
  {
      time_from_wireshark,
      Timestamp { seconds: 1_519_152_760, fraction: 1_328_210_046 },
      le = [0x78, 0x6E, 0x8C, 0x5A, 0x7E, 0xE0, 0x2A, 0x4F],
      be = [0x5A, 0x8C, 0x6E, 0x78, 0x4F, 0x2A, 0xE0, 0x7E]
  });
}
