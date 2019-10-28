use crate::structure::guid_prefix::GuidPrefix_t;
use speedy::{Readable, Writable};

/// This message is sent from an RTPS Writer to an RTPS Reader
/// to modify the GuidPrefix used to interpret the Reader entityIds
/// appearing in the Submessages that follow it.
#[derive(Debug, PartialEq, Readable, Writable)]
pub struct InfoDestination {
    /// Provides the GuidPrefix that should be used to reconstruct the GUIDs
    /// of all the RTPS Reader entities whose EntityIds appears
    /// in the Submessages that follow.
    pub guid_prefix: GuidPrefix_t,
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = InfoDestination,
    {
        info_destination,
        InfoDestination {
            guid_prefix: GuidPrefix_t {
                entityKey: [0x01, 0x02, 0x6D, 0x3F,
                            0x7E, 0x07, 0x00, 0x00,
                            0x01, 0x00, 0x00, 0x00]
            }
        },
        le = [0x01, 0x02, 0x6D, 0x3F,
              0x7E, 0x07, 0x00, 0x00,
              0x01, 0x00, 0x00, 0x00],
        be = [0x01, 0x02, 0x6D, 0x3F,
              0x7E, 0x07, 0x00, 0x00,
              0x01, 0x00, 0x00, 0x00]
    });
}
