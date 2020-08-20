use serde::{Serialize /*, Deserialize*/};

use crate::dds::traits::key::Keyed;

use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::serialization::cdrSerializer::{to_bytes};
use byteorder::{LittleEndian /*,BigEndian*/ };

use crate::structure::guid::EntityId;
use crate::structure::time::Timestamp;
use crate::structure::cache_change::ChangeKind;

// DDSData represets a serialized data sample with metadata
#[derive(Debug, PartialEq, Clone)]
pub struct DDSData {
  source_timestamp: Timestamp,
  pub change_kind: ChangeKind,
  reader_id: EntityId,
  writer_id: EntityId,
  value: Option<SerializedPayload>,
  pub value_key_hash: u64, // TODO: Is this used/needed ? If yes, please document its purpose here.
}

impl DDSData {
  pub fn new(payload: SerializedPayload) -> DDSData {
    DDSData {
      source_timestamp: Timestamp::from(time::get_time()),
      change_kind: ChangeKind::ALIVE,
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: Some(payload),
      value_key_hash: 0,
    }
  }

  // TODO: Rename this method, as it gets confued with the std library "From" trait method.
  pub fn from<D>(data: &D, source_timestamp: Option<Timestamp>) -> DDSData
  where
    D: Keyed + Serialize,
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
      change_kind: ChangeKind::ALIVE,
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: Some(serialized_payload),
      value_key_hash: 0,
    }
  }

  pub fn from_dispose<D>(_key: <D as Keyed>::K, source_timestamp: Option<Timestamp>) -> DDSData
  where
    D: Keyed,
  {
    let ts: Timestamp = match source_timestamp {
      Some(t) => t,
      None => Timestamp::from(time::get_time()),
    };

    // TODO: Serialize key

    DDSData {
      source_timestamp: ts,
      change_kind: ChangeKind::NOT_ALIVE_DISPOSED,
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      value: None, // TODO: Here we should place the serialized _key_, so that RTPS writer can send the
      // the DATA message indicating dispose
      value_key_hash: 0,
    }
  }

  fn serialize_data<D>(data: &D) -> Vec<u8>
  where
    D: Keyed + Serialize,
  {
    //let mut cdr = CDR_serializer::<LittleEndian>::new();
    //let mut serializer = erased_serde::Serializer::erase(&mut cdr);
    //let value = data.serialize(&mut cdr);
    // let value = to_little_endian_binary::<D>(&data);
    let value = match to_bytes::<D,LittleEndian>(data) {
      Ok(v) => v,
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

  pub fn value(&self) -> Option<SerializedPayload> {
    self.value.clone()
  }

  pub fn data(&self) -> Vec<u8> {
    match &self.value {
      Some(val) => (*val).value.clone(),
      None => Vec::new(),
    }
  }
}
