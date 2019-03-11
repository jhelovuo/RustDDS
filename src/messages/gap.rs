use crate::structure::entity_id::EntityId_t;
use crate::structure::sequence_number::SequenceNumber_t;
use crate::structure::sequence_number_set::SequenceNumberSet_t;
use speedy_derive::{Readable, Writable};

/// This Submessage is sent from an RTPS Writer to an RTPS Reader and
/// indicates to the RTPS Reader that a range of sequence numbers
/// is no longer relevant. The set may be a contiguous range of
/// sequence numbers or a specific set of sequence numbers.
#[derive(Debug, PartialEq, Readable, Writable)]
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

    /// Identifies the last sequence number in the interval of irrelevant
    /// sequence numbers.
    ///
    /// Identifies an additional list of sequence numbers that are
    /// irrelevant.
    pub gap_list: SequenceNumberSet_t,
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = Gap,
    {
        gap,
        Gap {
            reader_id: EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
            writer_id: EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
            gap_start: SequenceNumber_t::from(42),
            gap_list: SequenceNumberSet_t::new(SequenceNumber_t::from(7))
        },
        le = [0x00, 0x00, 0x03, 0xC7,
              0x00, 0x00, 0x03, 0xC2,
              0x00, 0x00, 0x00, 0x00,
              0x2A, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x07, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x03, 0xC7,
              0x00, 0x00, 0x03, 0xC2,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x2A,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x07,
              0x00, 0x00, 0x00, 0x00]
    });
}
