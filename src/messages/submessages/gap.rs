use crate::structure::sequence_number::{SequenceNumber, SequenceNumberSet};
use crate::{
  serialization::SubMessage, serialization::SubmessageBody, structure::guid::EntityId,
  messages::submessages::submessages::SubmessageHeader,
};
use enumflags2::BitFlags;
use log::error;
use speedy::{Readable, Writable};

use super::{
  submessage::EntitySubmessage, submessage_flag::GAP_Flags, submessage_kind::SubmessageKind,
};
/// This Submessage is sent from an RTPS Writer to an RTPS Reader and
/// indicates to the RTPS Reader that a range of sequence numbers
/// is no longer relevant. The set may be a contiguous range of
/// sequence numbers or a specific set of sequence numbers.
#[derive(Debug, PartialEq, Readable, Writable)]
pub struct Gap {
  /// Identifies the Reader Entity that is being informed of the
  /// irrelevance of a set of sequence numbers.
  pub reader_id: EntityId,

  /// Identifies the Writer Entity to which the range of sequence
  /// numbers applies.
  pub writer_id: EntityId,

  /// Identifies the first sequence number in the interval of
  /// irrelevant sequence numbers
  pub gap_start: SequenceNumber,

  /// Identifies the last sequence number in the interval of irrelevant
  /// sequence numbers.
  ///
  /// Identifies an additional list of sequence numbers that are
  /// irrelevant.
  pub gap_list: SequenceNumberSet,
}

impl Gap {
  pub fn create_submessage(self, flags: BitFlags<GAP_Flags>) -> Option<SubMessage> {
    let submessage_len = match self.write_to_vec() {
      Ok(bytes) => bytes.len() as u16,
      Err(e) => {
        error!("Reader couldn't write GAP to bytes: {}", e);
        return None
      }
    };

    Some(SubMessage {
      header: SubmessageHeader {
        kind: SubmessageKind::GAP,
        flags: flags.bits(),
        content_length: submessage_len,
      },
      body: SubmessageBody::Entity(EntitySubmessage::Gap(self, flags)),
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = Gap,
  {
      gap,
      Gap {
          reader_id: EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
          writer_id: EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
          gap_start: SequenceNumber::from(42),
          gap_list: SequenceNumberSet::new(SequenceNumber::from(7))
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
