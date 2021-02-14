use speedy::{Readable, Writable};
use std::convert::{From, TryFrom};
use std::ops::Div;
use serde::{Serialize, Deserialize};
use super::parameter_id::ParameterId;

use chrono;

#[derive(
  Debug,
  PartialEq,
  Eq,
  Hash,
  PartialOrd,
  Ord,
  Readable,
  Writable,
  Serialize,
  Deserialize,
  Copy,
  Clone,
)]

/// Duration for Qos and wire interoperability
/// Specified (as Duration_t) in RTPS spec 9.3.2
pub struct Duration {
  seconds: i32,
  fraction: u32, // unit is sec/2^32
}

impl Duration {
  pub const DURATION_ZERO: Duration = Duration {
    seconds: 0,
    fraction: 0,
  };

  pub const fn from_secs(secs: i32) -> Duration {
    Duration {
      seconds: secs, // loss of range here
      fraction: 0,
    }
  }

  pub fn from_frac_seconds(secs: f64) -> Duration {
    Duration {
      seconds: secs.trunc() as i32,
      fraction: (secs.fract().abs() * 32.0_f64.exp2()) as u32,
    }
  }

  pub const fn from_millis(millis: i64) -> Duration {
    let fraction = (((millis % 1000) << 32) / 1000) as u32; // correct formula?

    Duration {
      seconds: (millis / 1000) as i32,
      fraction,
    }
  }

  pub(crate) fn to_ticks(&self) -> i64 {
    ((self.seconds as i64) << 32) + (self.fraction as i64)
  }

  pub(crate) fn from_ticks(ticks: i64) -> Duration {
    Duration {
      seconds: (ticks >> 32) as i32,
      fraction: ticks as u32,
    }
  }

  /* DURATION_INVALID is not part of the spec. And it is also dangerous, as it is plausible someone could
  legitimately measure such an interval, and others would interpret it as "invalid".
  pub const DURATION_INVALID: Duration = Duration {
    seconds: -1,
    fraction: 0xFFFFFFFF,
  };*/

  pub const DURATION_INFINITE: Duration = Duration {
    seconds: 0x7FFFFFFF,
    fraction: 0xFFFFFFFF,
  };

  pub fn to_nanoseconds(&self) -> i64 {
    ((self.to_ticks() as i128 * 1_000_000_000) >> 32) as i64
  }

  pub fn from_std(duration: std::time::Duration) -> Self {
    Duration::from(duration)
  }

  pub fn to_std(&self) -> std::time::Duration {
    std::time::Duration::from(*self)
  }
}

impl From<Duration> for chrono::Duration {
  fn from(d: Duration) -> chrono::Duration {
    chrono::Duration::nanoseconds(d.to_nanoseconds())
  }
}

impl From<std::time::Duration> for Duration {
  fn from(duration: std::time::Duration) -> Self {
    Duration {
      seconds: duration.as_secs() as i32,
      fraction: (((duration.subsec_nanos() as u64) << 32) / 1_000_000_000) as u32,
    }
  }
}

impl From<Duration> for std::time::Duration {
  fn from(d: Duration) -> std::time::Duration {
    std::time::Duration::from_nanos(
      u64::try_from(d.to_nanoseconds()).unwrap_or(0), // saturate to zero, becaues std::time::Duraiton is unsigned
    )
  }
}

impl Div<i64> for Duration {
  type Output = Self;

  fn div(self, rhs: i64) -> Duration {
    Duration::from_ticks(self.to_ticks() / rhs)
  }
}


#[derive(Serialize, Deserialize)]
pub struct DurationData {
  parameter_id: ParameterId,
  parameter_length: u16,
  duration: Duration,
}

impl DurationData {
  pub fn from(duration: Duration) -> DurationData {
    DurationData {
      parameter_id: ParameterId::PID_PARTICIPANT_LEASE_DURATION,
      parameter_length: 8,
      duration: duration.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = Duration,
  {
      duration_zero,
      Duration::DURATION_ZERO,
      le = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
  },
  {
      duration_infinite,
      Duration::DURATION_INFINITE,
      le = [0xFF, 0xFF, 0xFF, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF],
      be = [0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  },
  {
      duration_current_empty_fraction,
      Duration { seconds: 1_537_045_491, fraction: 0 },
      le = [0xF3, 0x73, 0x9D, 0x5B, 0x00, 0x00, 0x00, 0x00],
      be = [0x5B, 0x9D, 0x73, 0xF3, 0x00, 0x00, 0x00, 0x00]
  },
  {
      duration_from_wireshark,
      Duration { seconds: 1_519_152_760, fraction: 1_328_210_046 },
      le = [0x78, 0x6E, 0x8C, 0x5A, 0x7E, 0xE0, 0x2A, 0x4F],
      be = [0x5A, 0x8C, 0x6E, 0x78, 0x4F, 0x2A, 0xE0, 0x7E]
  });

  const NANOS_PER_SEC: u64 = 1_000_000_000;

  #[test]
  fn convert_from_duration() {
    let duration = std::time::Duration::from_nanos(1_519_152_761 * NANOS_PER_SEC + 328_210_046);
    let duration: Duration = duration.into();

    assert_eq!(
      duration,
      Duration {
        seconds: 1_519_152_761,
        fraction: 1_409_651_413,
      }
    );
  }

  #[test]
  fn convert_to_duration() {
    let duration = Duration {
      seconds: 1_519_152_760,
      fraction: 1_328_210_046,
    };
    let duration: std::time::Duration = duration.into();

    assert_eq!(
      duration,
      std::time::Duration::from_nanos(1_519_152_760 * NANOS_PER_SEC + 309_247_999)
    );
  }
}
