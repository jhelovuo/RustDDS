use common::entity_id::{EntityId_t};
use common::sequence_number::{SequenceNumber_t};
use common::parameter::{ParameterList};

use message::submessage_header::{SubmessageHeader};
use message::validity_trait::Validity;

/// This Submessage is sent from an RTPS Writer (NO_KEY or WITH_KEY)
/// to an RTPS Reader (NO_KEY or WITH_KEY)
///
/// The Submessage notifies the RTPS Reader of a change to
/// a data-object belonging to the RTPS Writer. The possible changes
/// include both changes in value as well as changes to the lifecycle
/// of the data-object.
struct Data {
    submessage_header: SubmessageHeader,
    reader_id: EntityId_t,
    writer_id: EntityId_t,
    writer_sn: SequenceNumber_t,
    inline_qos: ParameterList,
    // serialized_payload: SerializedPayload // TODO: add type
}

impl Data {
    /// Indicates endianness. Returns true if big-endian,
    /// false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.submessage_header.flags.flags[0]
    }

    /// Indicates to the Reader the presence of a ParameterList
    /// containing QoS parameters that should be used to interpret
    /// the message
    pub fn inline_qos_flag(&self) -> bool {
        self.submessage_header.flags.flags[1]
    }

    /// Indicates to the Reader that the dataPayload submessage element
    /// contains the serialized value of the data-object
    pub fn data_flag(&self) -> bool {
        self.submessage_header.flags.flags[2]
    }

    /// Indicates to the Reader that the dataPayload submessage element
    /// contains the serialized value of the key of the data-object.
    pub fn key_flag(&self) -> bool {
        self.submessage_header.flags.flags[3]
    }
}

impl Validity for Data {
    fn valid(&self) -> bool {
        // TODO: finish validity self.writer_sn >= 0 && self.inline_qos.valid()
        // TODO: add D=1 and K=1 is an invalid combination in this version of the protocol.
        unimplemented!();
    }
}
