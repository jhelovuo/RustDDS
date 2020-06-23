use crate::structure::guid::EntityId;
use crate::structure::sequence_number::SequenceNumber;
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
  pub reader_id: EntityId,

  /// Identifies the Writer Entity to which the range of sequence
  /// numbers applies.
  pub writer_id: EntityId,

  /// Identifies the first (lowest) sequence number that is available in
  /// the Writer.
  pub first_sn: SequenceNumber,

  /// Identifies the last (highest) sequence number that is available in
  /// the Writer.
  pub last_sn: SequenceNumber,

  /// A counter that is increm ented each time a new Heartbeat
  /// message is sent.
  ///
  /// Provides the means for a Reader to detect duplicate Heartbeat
  /// messages that can result from the presence of redundant
  /// communication paths.
  pub count: i32,
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = Heartbeat,
  {
      heartbeat,
      Heartbeat {
          reader_id: EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
          writer_id: EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
          first_sn: SequenceNumber::from(42),
          last_sn: SequenceNumber::from(7),
          count: 9,
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
