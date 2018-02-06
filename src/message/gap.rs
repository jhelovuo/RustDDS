use common::submessage_flag::{SubmessageFlag};
use common::entity_id::{EntityId_t};
use common::sequence_number::{SequenceNumber_t, SequenceNumberSet_t};

use message::submessage_header::{SubmessageHeader};
use message::validity_trait::Validity;

/// This Submessage is sent from an RTPS Writer to an RTPS Reader and
/// indicates to the RTPS Reader that a range of sequence numbers
/// is no longer relevant. The set may be a contiguous range of
/// sequence numbers or a specific set of sequence numbers.
struct Gap {
    submessage_header: SubmessageHeader,
    /// Identifies the Reader Entity that is being informed of the
    /// irrelevance of a set of sequence numbers.
    reader_id: EntityId_t,
    /// Identifies the Writer Entity to which the range of sequence
    /// numbers applies.
    writer_id: EntityId_t,
    /// Identifies the first sequence number in the interval of
    /// irrelevant sequence numbers
    gap_start: SequenceNumber_t,
    /// Identifies the last sequence number in the interval of irrelevant sequence numbers.
    ///
    /// Identifies an additional list of sequence numbers that are
    /// irrelevant.
    gap_list: SequenceNumberSet_t
}

impl Gap {
    /// Indicates endianness. Returns true if big-endian, false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x01 != 0
    }
}

impl Validity for Gap {
    fn valid(&self) -> bool {
        self.gap_list.valid() && self.gap_start.value() > 0 // TODO: check value to operators
    }
}
