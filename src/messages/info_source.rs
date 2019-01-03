use crate::messages::protocol_version::ProtocolVersion_t;
use crate::messages::vendor_id::VendorId_t;
use crate::structure::guid_prefix::GuidPrefix_t;

/// This message modifies the logical source of the Submessages
/// that follow.
#[derive(Debug, PartialEq, Readable, Writable)]
pub struct InfoSource {
    /// Indicates the protocol used to encapsulate subsequent Submessages
    pub protocol_version: ProtocolVersion_t,

    /// Indicates the VendorId of the vendor that
    /// encapsulated subsequent Submessage
    pub vendor_id: VendorId_t,

    /// Identifies the Participant that is the container of the RTPS Writer
    /// entities that are the source of the Submessages that follow
    pub guid_prefix: GuidPrefix_t,
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = InfoSource,
    {
        info_source_empty,
        InfoSource {
            protocol_version: ProtocolVersion_t::PROTOCOLVERSION_2_2,
            vendor_id: VendorId_t {
                vendorId: [0xFF, 0xAA]
            },
            guid_prefix: GuidPrefix_t {
                entityKey: [0x01, 0x02, 0x6D, 0x3F,
                            0x7E, 0x07, 0x00, 0x00,
                            0x01, 0x00, 0x00, 0x00]
            }
        },
        le = [0x02, 0x02, 0xFF, 0xAA,
              0x01, 0x02, 0x6D, 0x3F,
              0x7E, 0x07, 0x00, 0x00,
              0x01, 0x00, 0x00, 0x00],
        be = [0x02, 0x02, 0xFF, 0xAA,
              0x01, 0x02, 0x6D, 0x3F,
              0x7E, 0x07, 0x00, 0x00,
              0x01, 0x00, 0x00, 0x00]
    });
}
