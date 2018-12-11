use crate::common::entity_id::EntityId_t;
use crate::common::parameter::ParameterList;
use crate::common::sequence_number::SequenceNumber_t;

use crate::message::submessage_header::SubmessageHeader;
use crate::message::validity_trait::Validity;

/// This Submessage is sent from an RTPS Writer (NO_KEY or WITH_KEY)
/// to an RTPS Reader (NO_KEY or WITH_KEY)
///
/// The Submessage notifies the RTPS Reader of a change to
/// a data-object belonging to the RTPS Writer. The possible changes
/// include both changes in value as well as changes to the lifecycle
/// of the data-object.
struct Data {
    submessage_header: SubmessageHeader,
    pub reader_id: EntityId_t,
    pub writer_id: EntityId_t,
    pub writer_sn: SequenceNumber_t,
    pub inline_qos: ParameterList,
    // serialized_payload: SerializedPayload // TODO: add type
}

impl Data {
    /// Indicates endianness. Returns true if big-endian,
    /// false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x01 != 0
    }

    /// Indicates to the Reader the presence of a ParameterList
    /// containing QoS parameters that should be used to interpret
    /// the message
    pub fn inline_qos_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x02 != 0
    }

    /// Indicates to the Reader that the dataPayload submessage element
    /// contains the serialized value of the data-object
    pub fn data_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x04 != 0
    }

    /// Indicates to the Reader that the dataPayload submessage element
    /// contains the serialized value of the key of the data-object.
    pub fn key_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x08 != 0
    }
}

impl Validity for Data {
    fn valid(&self) -> bool {
        self.writer_sn.low >= 0 &&
        // TODO: self.inline_qos.valid() &&
            !(self.data_flag() && self.key_flag())
    }
}
