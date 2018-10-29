extern crate rtps;
extern crate time;

use self::rtps::common::topic_kind::{TopicKind_t};

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
