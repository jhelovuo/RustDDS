#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct GuidPrefix_t {
    pub entityKey: [u8; 12],
}

pub const GUIDPREFIX_UNKNOWN: GuidPrefix_t = GuidPrefix_t { entityKey: [0x00; 12] };

impl Default for GuidPrefix_t {
    fn default() -> GuidPrefix_t {
        GuidPrefix_t {
            entityKey: [0x00; 12]
        }
    }
}
