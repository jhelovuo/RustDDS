use crate::{
  messages::submessages::submessage_elements::serialized_payload::SerializedPayload,
  messages::submessages::submessages::{RepresentationIdentifier, Data},
  structure::guid::EntityId,
  structure::sequence_number::SequenceNumber,
};

// Different properties that do not belong to the actual library but make testing easier.

impl Default for SerializedPayload {
  fn default() -> SerializedPayload {
    SerializedPayload::new(RepresentationIdentifier::CDR_LE, b"fake data".to_vec())
  }
}

impl Default for Data {
  fn default() -> Self {
    Data {
      reader_id: EntityId::default(),
      writer_id: EntityId::default(),
      writer_sn: SequenceNumber::default(),
      inline_qos: None,
      serialized_payload: Some(SerializedPayload::default()),
    }
  }
}
