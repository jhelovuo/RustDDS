use crate::common::submessage_flag::{SubmessageFlag};
use crate::common::entity_id::{EntityId_t};
use crate::common::sequence_number::{SequenceNumber_t};
use crate::common::count::{Count_t};

use crate::message::submessage_header::{SubmessageHeader};
use crate::message::validity_trait::Validity;

/// This Submessage is sent from an RTPS Writer to an RTPS Reader and
/// indicates to the RTPS Reader that a range of sequence numbers
/// is no longer relevant. The set may be a contiguous range of
/// sequence numbers or a specific set of sequence numbers.
struct HeartBeat {
    submessage_header: SubmessageHeader,

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
    pub count: Count_t
}

impl HeartBeat {
    /// Indicates endianness. Returns true if big-endian, false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x01 != 0
    }

    /// Indicates whether the Reader is required to respond to the
    /// Heartbeat or if it is just an advisory heartbeat
    pub fn final_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x02 != 0
    }

    /// Indicates that the DDS DataWriter associated with the RTPS
    /// Writer of the message has manually asserted its LIVELINESS
    pub fn liveliness_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x04 != 0
    }
}

impl Validity for HeartBeat {
    fn valid(&self) -> bool {
        self.first_sn.value() > 0 &&
            self.last_sn.value() > 0 &&
            self.last_sn.value() >= self.first_sn.value()
    }
}
