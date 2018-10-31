use crate::enum_number;

enum_number_i32!(ReliabilityKind_t {
    BEST_EFFORT = 1,
    RELIABLE = 3,
});

#[cfg(test)]
mod tests {
    use super::*;

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
}
