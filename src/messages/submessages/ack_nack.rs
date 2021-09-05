use crate::{
  serialization::SubMessage, serialization::SubmessageBody, structure::guid::EntityId,
  messages::submessages::submessages::SubmessageHeader,
};
use crate::structure::sequence_number::SequenceNumberSet;
use enumflags2::BitFlags;
use log::error;
use speedy::{Readable, Writable};

use super::{
  submessage::EntitySubmessage, submessage_flag::ACKNACK_Flags, submessage_kind::SubmessageKind,
};

/// This Submessage is used to communicate the state of a Reader to a
/// Writer.
///
/// The Submessage allows the Reader to inform the Writer about
/// the sequence numbers it has received and which ones it is still
/// missing. This Submessage can be used to do both positive
/// and negative acknowledgments
#[derive(Debug, PartialEq, Clone, Readable, Writable)]
pub struct AckNack {
  /// Identifies the Reader entity that acknowledges receipt of certain
  /// sequence numbers and/or requests to receive certain sequence numbers.
  pub reader_id: EntityId,

  /// Identifies the Writer entity that is the target of the AckNack message.
  /// This is the Writer Entity that is being asked to re-send some sequence
  /// numbers or is being informed of the reception of certain sequence
  /// numbers.
  pub writer_id: EntityId,

  /// Communicates the state of the reader to the writer.
  /// All sequence numbers up to the one prior to readerSNState.base
  /// are confirmed as received by the reader. The sequence numbers that
  /// appear in the set indicate missing sequence numbers on the reader
  /// side. The ones that do not appear in the set are undetermined (could
  /// be received or not).
  pub reader_sn_state: SequenceNumberSet,

  /// A counter that is incremented each time a new AckNack message is sent.
  /// Provides the means for a Writer to detect duplicate AckNack messages
  /// that can result from the presence of redundant communication paths.
  pub count: i32,
}

impl AckNack {
  pub fn create_submessage(self, flags: BitFlags<ACKNACK_Flags>) -> Option<SubMessage> {
    let submessage_len = match self.write_to_vec() {
      Ok(bytes) => bytes.len() as u16,
      Err(e) => {
        error!("Reader couldn't write acknack to bytes. Error: {}", e);
        return None;
      }
    };

    let acknack_header = SubmessageHeader {
      kind: SubmessageKind::ACKNACK,
      flags: flags.bits(),
      content_length: submessage_len,
    };

    Some(SubMessage {
      header: acknack_header,
      body: SubmessageBody::Entity(EntitySubmessage::AckNack(self, flags)),
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::structure::sequence_number::SequenceNumber;

  serialization_test!( type = AckNack,
  {
      acknack,
      AckNack {
          reader_id: EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
          writer_id: EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
          reader_sn_state: SequenceNumberSet::new_empty(SequenceNumber::from(0)),
          count: 1,
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
