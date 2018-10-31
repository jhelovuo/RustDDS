/// Type used to represent the identity of a data-object whose changes in value are
/// communicated by the RTPS protocol.
#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Ord, Eq)]
pub struct InstanceHandle_t {
    pub entityKey: [u8; 16],
}

impl Default for InstanceHandle_t {
    fn default() -> InstanceHandle_t {
        InstanceHandle_t {
            entityKey: [0x00; 16]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    assert_ser_de!(
        {
            instance_handle_default,
            InstanceHandle_t::default(),
            le = [0x00; 16],
            be = [0x00; 16]
        },
        {
            instance_handle_endianness_insensitive,
            InstanceHandle_t {
                entityKey: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                            0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
            },
            le = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                  0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
            be = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                  0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
        }
    );
}
