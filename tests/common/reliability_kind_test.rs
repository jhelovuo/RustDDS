extern crate rtps;
extern crate time;

use self::rtps::common::reliability_kind::{ReliabilityKind_t};

assert_ser_de!({
    reliability_kind_best_effort,
    ReliabilityKind_t::BEST_EFFORT,
    le = [0x01, 0x00, 0x00, 0x00],
    be = [0x00, 0x00, 0x00, 0x01]
});

assert_ser_de!({
    reliability_kind_reliable,
    ReliabilityKind_t::RELIABLE,
    le = [0x03, 0x00, 0x00, 0x00],
    be = [0x00, 0x00, 0x00, 0x03]
});
