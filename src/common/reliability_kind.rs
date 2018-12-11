#[derive(Debug, Eq, PartialEq, Readable, Writable)]
pub enum ReliabilityKind_t {
    BEST_EFFORT = 1,
    RELIABLE = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = ReliabilityKind_t,
        {
            reliability_kind_best_effort,
            ReliabilityKind_t::BEST_EFFORT,
            le = [0x01, 0x00, 0x00, 0x00],
            be = [0x00, 0x00, 0x00, 0x01]
        },
        {
            reliability_kind_reliable,
            ReliabilityKind_t::RELIABLE,
            le = [0x03, 0x00, 0x00, 0x00],
            be = [0x00, 0x00, 0x00, 0x03]
        });
}
