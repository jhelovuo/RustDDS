use crate::common::count::Count_t;
use crate::common::entity_id::EntityId_t;
use crate::common::fragment_number::FragmentNumber_t;
use crate::common::sequence_number::SequenceNumber_t;

/// When fragmenting data and until all fragments are available, the
/// HeartbeatFrag Submessage is sent from an RTPS Writer to an RTPS Reader to
/// communicate which fragments the Writer has available. This enables reliable
/// communication at the fragment level.
///
/// Once all fragments are available, a regular Heartbeat message is used.
#[derive(Debug, PartialEq)]
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
