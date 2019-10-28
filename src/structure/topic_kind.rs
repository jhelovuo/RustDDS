use speedy::{Readable, Writable};

#[derive(Debug, PartialEq, Eq, Readable, Writable)]
pub struct TopicKind_t {
    value: u32,
}

impl TopicKind_t {
    pub const NO_KEY: TopicKind_t = TopicKind_t { value: 1 };
    pub const WITH_KEY: TopicKind_t = TopicKind_t { value: 2 };
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!(type = TopicKind_t,
    {
        topic_kind_no_key,
        TopicKind_t::NO_KEY,
        le = [0x01, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x01]
    },
    {
        topic_kind_with_key,
        TopicKind_t::WITH_KEY,
        le = [0x02, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x02]
    });
}
