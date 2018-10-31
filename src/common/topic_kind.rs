use crate::enum_number;

enum_number_i32!(TopicKind_t {
    NO_KEY = 1,
    WITH_KEY = 2,
});

#[cfg(test)]
mod tests {
    use super::*;

    assert_ser_de!({
        topic_kind_no_key,
        TopicKind_t::NO_KEY,
        le = [0x01, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x01]
    });

    assert_ser_de!({
        topic_kind_with_key,
        TopicKind_t::WITH_KEY,
        le = [0x02, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x00, 0x02]
    });
}
