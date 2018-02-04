use common::submessage_flag::{SubmessageFlag};
use common::entity_id::{EntityId_t};
use common::sequence_number::{SequenceNumberSet_t};
use common::count::{Count_t};

use message::submessage_header::{SubmessageHeader};
use message::validity_trait::Validity;

/// This Submessage is used to communicate the state of a Reader to a
/// Writer.
///
/// The Submessage allows the Reader to inform the Writer about
/// the sequence numbers it has received and which ones it is still
/// missing. This Submessage can be used to do both positive
/// and negative acknowledgments
struct AckNack {
    submessage_header: SubmessageHeader,
    reader_id: EntityId_t,
    writer_id: EntityId_t,
    reader_sn_state: SequenceNumberSet_t,
    count: Count_t
}

impl AckNack {
    /// Indicates endianness. Returns true if big-endian, false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.submessage_header.flags.flags[7]
    }
}

impl Validity for AckNack {
    fn valid(&self) -> bool {
        self.reader_sn_state.valid()
    }
}
