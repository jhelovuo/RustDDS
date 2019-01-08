use speedy_derive::{Readable, Writable};

#[derive(Debug, PartialEq, Eq, Readable, Writable)]
pub struct ReliabilityKind_t {
    value: u32,
}

impl ReliabilityKind_t {
    pub const BEST_EFFORT: ReliabilityKind_t = ReliabilityKind_t { value: 1 };
    pub const RELIABLE: ReliabilityKind_t = ReliabilityKind_t { value: 3 };
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
