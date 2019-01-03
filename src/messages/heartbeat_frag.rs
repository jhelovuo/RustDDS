use crate::messages::fragment_number::FragmentNumber_t;
use crate::structure::count::Count_t;
use crate::structure::entity_id::EntityId_t;
use crate::structure::sequence_number::SequenceNumber_t;

/// When fragmenting data and until all fragments are available, the
/// HeartbeatFrag Submessage is sent from an RTPS Writer to an RTPS Reader to
/// communicate which fragments the Writer has available. This enables reliable
/// communication at the fragment level.
///
/// Once all fragments are available, a regular Heartbeat message is used.
#[derive(Debug, PartialEq, Readable, Writable)]
pub struct HeartbeatFrag {
    /// Identifies the Reader Entity that is being informed of the availability
    /// of fragments. Can be set to ENTITYID_UNKNOWN to indicate all readers for
    /// the writer that sent the message.
    pub reader_id: EntityId_t,

    /// Identifies the Writer Entity that sent the Submessage.
    pub writer_id: EntityId_t,

    /// Identifies the sequence number of the data change for which fragments
    /// are available.
    pub writer_sn: SequenceNumber_t,

    /// All fragments up to and including this last (highest) fragment are
    /// available on the Writer for the change identified by writerSN.
    pub last_fragment_num: FragmentNumber_t,

    /// A counter that is incremented each time a new HeartbeatFrag message is
    /// sent. Provides the means for a Reader to detect duplicate HeartbeatFrag
    /// messages that can result from the presence of redundant communication
    /// paths.
    pub count: Count_t,
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = HeartbeatFrag,
    {
        heartbeat_frag,
        HeartbeatFrag {
            reader_id: EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
            writer_id: EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
            writer_sn: SequenceNumber_t {
                high: 42,
                low: 7
            },
            last_fragment_num: FragmentNumber_t {
                value: 99
            },
            count: Count_t {
                value: 6
            }
        },
        le = [0x00, 0x00, 0x03, 0xC7,
              0x00, 0x00, 0x03, 0xC2,
              0x2A, 0x00, 0x00, 0x00,
              0x07, 0x00, 0x00, 0x00,
              0x63, 0x00, 0x00, 0x00,
              0x06, 0x00, 0x00, 0x00],
        be = [0x00, 0x00, 0x03, 0xC7,
              0x00, 0x00, 0x03, 0xC2,
              0x00, 0x00, 0x00, 0x2A,
              0x00, 0x00, 0x00, 0x07,
              0x00, 0x00, 0x00, 0x63,
              0x00, 0x00, 0x00, 0x06]
    });
}
