use crate::messages::submessage_elements::parameter_list::ParameterList;
use crate::messages::submessage_elements::serialized_payload::SerializedPayload;
use crate::structure::entity_id::EntityId_t;
use crate::structure::sequence_number::SequenceNumber_t;

/// This Submessage is sent from an RTPS Writer (NO_KEY or WITH_KEY)
/// to an RTPS Reader (NO_KEY or WITH_KEY)
///
/// The Submessage notifies the RTPS Reader of a change to
/// a data-object belonging to the RTPS Writer. The possible changes
/// include both changes in value as well as changes to the lifecycle
/// of the data-object.
#[derive(Debug, PartialEq)]
pub struct Data {
    /// Identifies the RTPS Reader entity that is being informed of the change to the data-object.
    pub reader_id: EntityId_t,

    /// Identifies the RTPS Writer entity that made the change to the data-object.
    pub writer_id: EntityId_t,

    /// Uniquely identifies the change and the relative order for all changes made by the RTPS Writer
    /// identified by the writerGuid. Each change gets a consecutive sequence number.
    /// Each RTPS Writer maintains is own sequence number.
    pub writer_sn: SequenceNumber_t,

    /// Contains QoS that may affect the interpretation of the message.
    /// Present only if the InlineQosFlag is set in the header.
    pub inline_qos: ParameterList,

    /// If the DataFlag is set, then it contains the encapsulation of
    /// the new value of the data-object after the change.
    /// If the KeyFlag is set, then it contains the encapsulation of
    /// the key of the data-object the message refers to.
    pub serialized_payload: SerializedPayload,
}
