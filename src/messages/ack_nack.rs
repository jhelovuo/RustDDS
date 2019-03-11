use crate::structure::count::Count_t;
use crate::structure::entity_id::EntityId_t;
use crate::structure::sequence_number_set::SequenceNumberSet_t;
use speedy_derive::{Readable, Writable};

/// This Submessage is used to communicate the state of a Reader to a
/// Writer.
///
/// The Submessage allows the Reader to inform the Writer about
/// the sequence numbers it has received and which ones it is still
/// missing. This Submessage can be used to do both positive
/// and negative acknowledgments
#[derive(Debug, PartialEq, Readable, Writable)]
pub struct AckNack {
    /// Identifies the Reader entity that acknowledges receipt of certain
    /// sequence numbers and/or requests to receive certain sequence numbers.
    pub reader_id: EntityId_t,

    /// Identifies the Writer entity that is the target of the AckNack message.
    /// This is the Writer Entity that is being asked to re-send some sequence
    /// numbers or is being informed of the reception of certain sequence
    /// numbers.
    pub writer_id: EntityId_t,

    /// Communicates the state of the reader to the writer.
    /// All sequence numbers up to the one prior to readerSNState.base
    /// are confirmed as received by the reader. The sequence numbers that
    /// appear in the set indicate missing sequence numbers on the reader
    /// side. The ones that do not appear in the set are undetermined (could
    /// be received or not).
    pub reader_sn_state: SequenceNumberSet_t,

    /// A counter that is incremented each time a new AckNack message is sent.
    /// Provides the means for a Writer to detect duplicate AckNack messages
    /// that can result from the presence of redundant communication paths.
    pub count: Count_t,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::sequence_number::SequenceNumber_t;

    serialization_test!( type = AckNack,
    {
        acknack,
        AckNack {
            reader_id: EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
            writer_id: EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
            reader_sn_state: SequenceNumberSet_t::new(SequenceNumber_t::from(0)),
            count: Count_t::from(1)
        },
        le = [0x00, 0x00, 0x03, 0xC7,
              0x00, 0x00, 0x03, 0xC2,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x01, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x03, 0xC7,
              0x00, 0x00, 0x03, 0xC2,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x01]
    });
}
