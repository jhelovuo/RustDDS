use crate::structure::guid::EntityId;
use crate::structure::sequence_number::*;
use speedy::{Readable, Writable};

/// The NackFrag Submessage is used to communicate the state of a Reader to a
/// Writer. When a data change is sent as a series of fragments, the NackFrag
/// Submessage allows the Reader to inform the Writer about specific fragment
/// numbers it is still missing.
///
/// This Submessage can only contain negative acknowledgements. Note this
/// differs from an AckNack Submessage, which includes both positive and
/// negative acknowledgements.
#[derive(Debug, PartialEq, Clone, Readable, Writable)]
pub struct NackFrag {
  ///  Identifies the Reader entity that requests to receive certain
  /// fragments.
  pub reader_id: EntityId,

  /// Identifies the Writer entity that is the target of the NackFrag message.
  /// This is the Writer Entity that is being asked to re-send some fragments.
  pub writer_id: EntityId,

  /// The sequence number for which some fragments are missing.
  pub writer_sn: SequenceNumber,

  /// Communicates the state of the reader to the writer.
  /// The fragment numbers that appear in the set indicate missing
  /// fragments on the reader side. The ones that do not appear in the set
  /// are undetermined (could have been received or not).
  pub fragment_number_state: FragmentNumberSet,

  /// A counter that is incremented each time a new NackFrag message is sent.
  /// Provides the means for a Writer to detect duplicate NackFrag messages
  /// that can result from the presence of redundant communication paths.
  pub count: i32,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::structure::sequence_number::FragmentNumber;

  serialization_test!( type = NackFrag,
  {
      nack_frag,
      NackFrag {
          reader_id: EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
          writer_id: EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
          writer_sn: SequenceNumber::from(42),
          fragment_number_state: FragmentNumberSet::new_empty(FragmentNumber::from(1000u32)),
          count: 6,
      },
      le = [0x00, 0x00, 0x03, 0xC7,
            0x00, 0x00, 0x03, 0xC2,
            0x00, 0x00, 0x00, 0x00,
            0x2A, 0x00, 0x00, 0x00,
            0xE8, 0x03, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x06, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x03, 0xC7,
            0x00, 0x00, 0x03, 0xC2,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x2A,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x06]
  });
}
