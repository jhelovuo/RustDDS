use speedy::{Readable, Writable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Readable, Writable, Serialize, Deserialize)]
pub struct ReliabilityKind(u32);

impl ReliabilityKind {
  pub const BEST_EFFORT: Self = Self(1);
  pub const RELIABLE: Self = Self(2);
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = ReliabilityKind,
  {
      reliability_kind_best_effort,
      ReliabilityKind::BEST_EFFORT,
      le = [0x01, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x01]
  },
  {
      reliability_kind_reliable,
      ReliabilityKind::RELIABLE,
      le = [0x02, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x02]
  });
}
