use crate::structure::count::Count_t;
use crate::structure::entity_id::EntityId_t;
use crate::structure::sequence_number::SequenceNumber_t;
use speedy::{Readable, Writable};

/// This Submessage is sent from an RTPS Writer to an RTPS Reader and
/// indicates to the RTPS Reader that a range of sequence numbers
/// is no longer relevant. The set may be a contiguous range of
/// sequence numbers or a specific set of sequence numbers.
#[derive(Debug, PartialEq, Readable, Writable)]
pub struct Heartbeat {
    /// Identifies the Reader Entity that is being informed of the
    /// availability of a set of sequence numbers.
    ///
    /// Can be set to ENTITYID_UNKNOWN to indicate all readers
    /// for the writer that sent the message.
    pub reader_id: EntityId_t,

    /// Identifies the Writer Entity to which the range of sequence
    /// numbers applies.
    pub writer_id: EntityId_t,

    /// Identifies the first (lowest) sequence number that is available in
    /// the Writer.
    pub first_sn: SequenceNumber_t,

    /// Identifies the last (highest) sequence number that is available in
    /// the Writer.
    pub last_sn: SequenceNumber_t,

    /// A counter that is increm ented each time a new Heartbeat
    /// message is sent.
    ///
    /// Provides the means for a Reader to detect duplicate Heartbeat
    /// messages that can result from the presence of redundant
    /// communication paths.
    pub count: Count_t,
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = Heartbeat,
    {
        heartbeat,
        Heartbeat {
            reader_id: EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
            writer_id: EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
            first_sn: SequenceNumber_t::from(42),
            last_sn: SequenceNumber_t::from(7),
            count: Count_t::from(9)
        },
        le = [0x00, 0x00, 0x03, 0xC7,
              0x00, 0x00, 0x03, 0xC2,
              0x00, 0x00, 0x00, 0x00,
              0x2A, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x07, 0x00, 0x00, 0x00,
              0x09, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x03, 0xC7,
              0x00, 0x00, 0x03, 0xC2,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x2A,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x07,
              0x00, 0x00, 0x00, 0x09]
    });
}
