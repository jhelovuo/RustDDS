#[derive(Debug, PartialEq, Readable, Writable)]
enum TopicKind_t {
    NO_KEY = 1,
    WITH_KEY = 2,
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
