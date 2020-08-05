use crate::dds::traits::datasample_trait::DataSampleTrait;
use std::sync::Arc;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::serialization::cdrSerializer::{CDR_serializer, Endianess};
use crate::structure::guid::EntityId;
use crate::structure::time::Timestamp;
use erased_serde;

#[derive(Debug, PartialEq, Clone)]
pub struct DDSData {
  source_timestamp: Timestamp,
  reader_id: EntityId,
  writer_id: EntityId,
  value: Arc<SerializedPayload>,
}

impl DDSData {
  pub fn new(payload: SerializedPayload) -> DDSData {
    DDSData {
      source_timestamp: Timestamp::from(time::get_time()),
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: Arc::new(payload),
    }
  }

  pub fn from_arc(payload: Arc<SerializedPayload>) -> DDSData {
    DDSData {
      source_timestamp: Timestamp::from(time::get_time()),
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: payload.clone(),
    }
  }

  pub fn from<D>(data: &D, source_timestamp: Option<Timestamp>) -> DDSData
  where
    D: DataSampleTrait,
  {
    let mut cdr = CDR_serializer::new(Endianess::LittleEndian);
    let mut serializer = erased_serde::Serializer::erase(&mut cdr);
    let value = data.erased_serialize(&mut serializer);
    // let value = to_little_endian_binary::<D>(&data);
    let value = match value {
      Ok(_) => cdr.buffer().clone(),
      // TODO: handle error
      _ => Vec::new(),
    };

    let ts: Timestamp = match source_timestamp {
      Some(t) => t,
      None => Timestamp::from(time::get_time()),
    };

    let mut serialized_payload = SerializedPayload::new();
    // TODO: read identifier
    serialized_payload.representation_identifier = 0;
    serialized_payload.representation_options = 0;
    serialized_payload.value = value;

    DDSData {
      source_timestamp: ts,
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: Arc::new(serialized_payload),
    }
  }

  pub fn reader_id(&self) -> &EntityId {
    &self.reader_id
  }

  pub fn set_reader_id(&mut self, reader_id: EntityId) {
    self.reader_id = reader_id;
  }

  pub fn writer_id(&self) -> &EntityId {
    &self.writer_id
  }

  pub fn set_writer_id(&mut self, writer_id: EntityId) {
    self.writer_id = writer_id;
  }

  pub fn value(&self) -> Arc<SerializedPayload> {
    self.value.clone()
  }

  pub fn data(&self) -> Vec<u8> {
    (*self.value).value.clone()
  }
}
