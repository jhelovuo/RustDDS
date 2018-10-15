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
