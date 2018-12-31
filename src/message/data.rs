use crate::common::parameter_list::ParameterList;
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
    pub reader_id: EntityId_t,
    pub writer_id: EntityId_t,
    pub writer_sn: SequenceNumber_t,
    pub inline_qos: ParameterList,
    // serialized_payload: SerializedPayload // TODO: add type
}
