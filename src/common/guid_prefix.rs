#[derive(Serialize, Deserialize, Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct GuidPrefix_t {
    pub entityKey: [u8; 12],
}

pub const GUIDPREFIX_UNKNOWN: GuidPrefix_t = GuidPrefix_t { entityKey: [0x00; 12] };

impl Default for GuidPrefix_t {
    fn default() -> GuidPrefix_t {
        GUIDPREFIX_UNKNOWN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    assert_ser_de!(
        {
            guid_prefix_unknown,
            GUIDPREFIX_UNKNOWN,
            le = [0x00; 12],
            be = [0x00; 12]
        },
        {
            guid_prefix_default,
            GuidPrefix_t::default(),
            le = [0x00; 12],
            be = [0x00; 12]
        },
        {
            guid_prefix_endianness_insensitive,
            GuidPrefix_t {
                entityKey: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
                            0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]
            },
            le = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
                  0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB],
            be = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
                  0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]
        }
    );
}
