use crate::dds::traits::key::{DefaultKey, Keyed};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::dds::traits::named::Named;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::serialization::cdrSerializer::to_little_endian_binary;
use crate::structure::guid::EntityId;

pub trait DDSDataTrait<'a>: Deserialize<'a> + Keyed {}

#[derive(Deserialize)]
pub struct DummyData {
  key: DefaultKey,
}

impl DummyData {
  pub fn default() -> DummyData {
    DummyData {
      key: DefaultKey::default(),
    }
  }
}

impl Named for DummyData {
  fn name() -> String {
    "DummyData".to_string()
  }
  fn identifier() -> u16 {
    0
  }
}

impl Keyed for DummyData {
  type K = DefaultKey;
  fn get_key(&self) -> &Self::K {
    &self.key
  }

  fn default() -> Self {
    Self {
      key: DefaultKey::default(),
    }
  }
}

impl<'a> DDSDataTrait<'a> for DummyData {}

#[derive(Debug, PartialEq, Clone)]
pub struct DDSData {
  key: DefaultKey,
  reader_id: EntityId,
  writer_id: EntityId,
  value: Arc<Vec<u8>>,
}

impl DDSData {
  pub fn new(payload: SerializedPayload) -> DDSData {
    DDSData {
      key: DefaultKey::random_key(),
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: Arc::new(payload.value),
    }
  }

  pub fn from<D>(data: D) -> DDSData
  where
    D: Keyed + Serialize,
  {
    let key = DefaultKey::random_key();
    let value = to_little_endian_binary::<D>(&data);
    let value = match value {
      Ok(val) => val,
      // TODO: handle error
      _ => Vec::new(),
    };
    DDSData {
      key,
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: Arc::new(value),
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

  pub fn data(&self) -> Arc<Vec<u8>> {
    self.value.clone()
  }
}

impl Keyed for DDSData {
  type K = DefaultKey;

  fn get_key(&self) -> &Self::K {
    &self.key
  }

  fn default() -> Self {
    Self {
      key: DefaultKey::default(),
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: Arc::new(Vec::new()),
    }
  }
}
