use crate::common::entity_id::EntityId_t;
use crate::common::sequence_number::{SequenceNumberSet_t, SequenceNumber_t};
use crate::message::validity_trait::Validity;

/// This Submessage is sent from an RTPS Writer to an RTPS Reader and
/// indicates to the RTPS Reader that a range of sequence numbers
/// is no longer relevant. The set may be a contiguous range of
/// sequence numbers or a specific set of sequence numbers.
#[derive(Debug, PartialEq)]
pub struct Gap {
    /// Identifies the Reader Entity that is being informed of the
    /// irrelevance of a set of sequence numbers.
    pub reader_id: EntityId_t,
    /// Identifies the Writer Entity to which the range of sequence
    /// numbers applies.
    pub writer_id: EntityId_t,
    /// Identifies the first sequence number in the interval of
    /// irrelevant sequence numbers
    pub gap_start: SequenceNumber_t,
    /// Identifies the last sequence number in the interval of irrelevant sequence numbers.
    ///
    /// Identifies an additional list of sequence numbers that are
    /// irrelevant.
    pub gap_list: SequenceNumberSet_t,
}
