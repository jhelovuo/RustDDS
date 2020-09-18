use speedy::{Readable, Writable};
use std::convert::From;
use std::time::Duration as TDuration;
use serde::{Serialize, Deserialize};
use super::parameter_id::ParameterId;

#[derive(
  Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Readable, Writable, Serialize, Deserialize, Copy, Clone,
)]
pub struct Duration {
  seconds: i32,
  fraction: u32,
}

impl Duration {
  pub const DURATION_ZERO: Duration = Duration {
    seconds: 0,
    fraction: 0,
  };
  pub const DURATION_INVALID: Duration = Duration {
    seconds: -1,
    fraction: 0xFFFFFFFF,
  };
  pub const DURATION_INFINITE: Duration = Duration {
    seconds: 0x7FFFFFFF,
    fraction: 0xFFFFFFFF,
  };
}

impl From<TDuration> for Duration {
  fn from(duration: TDuration) -> Self {
    Duration {
      seconds: duration.as_secs() as i32,
      fraction: duration.subsec_nanos() as u32,
    }
  }
}

impl From<Duration> for TDuration {
  fn from(duration: Duration) -> Self {
    TDuration::new(duration.seconds as u64, duration.fraction)
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
      duration_invalid,
      Duration::DURATION_INVALID,
      le = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
      be = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
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
    let duration = TDuration::from_nanos(1_519_152_761 * NANOS_PER_SEC + 328_210_046);
    let duration: Duration = duration.into();

    assert_eq!(
      duration,
      Duration {
        seconds: 1_519_152_761,
        fraction: 328_210_046,
      }
    );
  }

  #[test]
  fn convert_to_duration() {
    let duration = Duration {
      seconds: 1_519_152_760,
      fraction: 1_328_210_046,
    };
    let duration: TDuration = duration.into();

    assert_eq!(
      duration,
      TDuration::from_nanos(1_519_152_760 * NANOS_PER_SEC + 1_328_210_046)
    );
  }
}
