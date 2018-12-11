use crate::common::count::Count_t;
use crate::common::entity_id::EntityId_t;
use crate::common::sequence_number::{SequenceNumberSet_t, SequenceNumber_t};
use crate::common::submessage_flag::SubmessageFlag;

use crate::message::submessage_header::{SubmessageHeader, SubmessageKind};
use crate::message::validity_trait::Validity;

use crate::common::entity_id::*;

/// This Submessage is used to communicate the state of a Reader to a
/// Writer.
///
/// The Submessage allows the Reader to inform the Writer about
/// the sequence numbers it has received and which ones it is still
/// missing. This Submessage can be used to do both positive
/// and negative acknowledgments
#[derive(Debug, PartialEq, Readable, Writable)]
struct AckNack {
    submessage_header: SubmessageHeader,
    reader_id: EntityId_t,
    writer_id: EntityId_t,
    reader_sn_state: SequenceNumberSet_t,
    count: Count_t,
}

impl AckNack {
    /// Indicates endianness. Returns true if big-endian, false if little-endian
    pub fn endianness_flag(&self) -> bool {
        self.submessage_header.flags.flags & 0x01 != 0
    }
}

impl Validity for AckNack {
    fn valid(&self) -> bool {
        self.reader_sn_state.valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = AckNack,
    {
        acknack,
        AckNack {
            submessage_header: SubmessageHeader {
                submessage_id: SubmessageKind::ACKNACK,
                flags: SubmessageFlag {
                    flags: 0x01
                },
                submessage_length: 24
            },
            reader_id: ENTITY_SEDP_BUILTIN_PUBLICATIONS_READER,
            writer_id: ENTITY_SEDP_BUILTIN_PUBLICATIONS_WRITER,
            reader_sn_state: SequenceNumberSet_t::new(SequenceNumber_t {
                high: 0,
                low: 0
            }),
            count: Count_t {
                value: 1
            }
        },
        le = [0x06, 0x01, 0x18, 0x00,
              0x00, 0x00, 0x03, 0xC7,
              0x00, 0x00, 0x03, 0xC2,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x01, 0x00, 0x00, 0x00],
        be = [0x06, 0x01, 0x00, 0x18,
              0x00, 0x00, 0x03, 0xC7,
              0x00, 0x00, 0x03, 0xC2,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x01]
    });
}
