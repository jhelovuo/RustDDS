use speedy::{Readable, Writable};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Readable, Writable)]
pub struct TopicKind {
  value: u32,
}

impl TopicKind {
  pub const NO_KEY: TopicKind = TopicKind { value: 1 };
  pub const WITH_KEY: TopicKind = TopicKind { value: 2 };
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!(type = TopicKind,
  {
      topic_kind_no_key,
      TopicKind::NO_KEY,
      le = [0x01, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x01]
  },
  {
      topic_kind_with_key,
      TopicKind::WITH_KEY,
      le = [0x02, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x02]
  });
}
