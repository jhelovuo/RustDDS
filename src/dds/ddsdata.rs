use crate::dds::traits::datasample_trait::DataSampleTrait;
use std::sync::Arc;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::serialization::cdrSerializer::{CDR_serializer, Endianess};
use crate::structure::guid::EntityId;
use crate::structure::time::Timestamp;
use crate::structure::instance_handle::InstanceHandle;
use crate::structure::cache_change::ChangeKind;
use erased_serde;

#[derive(Debug, PartialEq, Clone)]
pub struct DDSData {
  source_timestamp: Timestamp,
  pub instance_key: InstanceHandle,
  pub change_kind: ChangeKind,
  reader_id: EntityId,
  writer_id: EntityId,
  value: Option<Arc<SerializedPayload>>,
  pub value_key_hash: u64, 
}

impl DDSData {
  pub fn new(instance_key: InstanceHandle, payload: SerializedPayload) -> DDSData {
    DDSData {
      source_timestamp: Timestamp::from(time::get_time()),
      instance_key,
      change_kind: ChangeKind::ALIVE,
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: Some(Arc::new(payload)),
      value_key_hash : 0
    }
  }

  pub fn from_arc(instance_key: InstanceHandle, payload: Arc<SerializedPayload>) -> DDSData {
    DDSData {
      source_timestamp: Timestamp::from(time::get_time()),
      instance_key,
      change_kind: ChangeKind::ALIVE,
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: Some(payload.clone()),
      value_key_hash :0
    }
  }

  pub fn from<D>(
    instance_key: InstanceHandle,
    data: &D,
    source_timestamp: Option<Timestamp>,
  ) -> DDSData
  where
    D: DataSampleTrait,
  {
    let value = DDSData::serialize_data(data);

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
      instance_key,
      change_kind: ChangeKind::ALIVE,
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: Some(Arc::new(serialized_payload)),
      value_key_hash : 0
    }
  }

  pub fn from_dispose<D>(
    instance_key: InstanceHandle,
    _data: &D,
    source_timestamp: Option<Timestamp>,
  ) -> DDSData {
    let ts: Timestamp = match source_timestamp {
      Some(t) => t,
      None => Timestamp::from(time::get_time()),
    };

    DDSData {
      source_timestamp: ts,
      instance_key,
      change_kind: ChangeKind::NOT_ALIVE_DISPOSED,
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: None,
      value_key_hash : 0,
    }
  }

  fn serialize_data<D>(data: &D) -> Vec<u8>
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
    value
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

  pub fn value(&self) -> Option<Arc<SerializedPayload>> {
    self.value.clone()
  }

  pub fn data(&self) -> Vec<u8> {
    match &self.value {
      Some(val) => (*val).value.clone(),
      None => Vec::new(),
    }
  }
}
