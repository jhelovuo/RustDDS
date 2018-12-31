use crate::structure::count::Count_t;
use crate::structure::entity_id::EntityId_t;
use crate::structure::sequence_number::SequenceNumberSet_t;

/// This Submessage is used to communicate the state of a Reader to a
/// Writer.
///
/// The Submessage allows the Reader to inform the Writer about
/// the sequence numbers it has received and which ones it is still
/// missing. This Submessage can be used to do both positive
/// and negative acknowledgments
#[derive(Debug, PartialEq, Readable, Writable)]
pub struct AckNack {
    pub reader_id: EntityId_t,
    pub writer_id: EntityId_t,
    pub reader_sn_state: SequenceNumberSet_t,
    pub count: Count_t,
}
