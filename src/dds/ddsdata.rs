use crate::dds::traits::key::{Keyed, DefaultKey};
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::messages::submessages::submessage_elements::parameter_list::ParameterList;
use crate::messages::submessages::data::Data;
use crate::structure::guid::EntityId;
use crate::structure::sequence_number::SequenceNumber;

#[derive(Debug, PartialEq)]
pub struct DDSData {
  key: DefaultKey,
  pub reader_id: EntityId,
  pub writer_id: EntityId,
  pub writer_sn: SequenceNumber,
  inline_qos: Option<ParameterList>,
  data: SerializedPayload,
}

impl DDSData {
  pub fn new(key: DefaultKey, data: SerializedPayload) -> DDSData {
    DDSData {
      key,
      reader_id: EntityId::ENTITYID_UNKNOWN,
      writer_id: EntityId::ENTITYID_UNKNOWN,
      writer_sn: SequenceNumber::from(0),
      inline_qos: None,
      data,
    }
  }

  pub fn from(data: Data) -> DDSData {
    DDSData {
      key: DefaultKey::random_key(),
      reader_id: data.reader_id,
      writer_id: data.writer_id,
      writer_sn: data.writer_sn,
      inline_qos: data.inline_qos,
      data: data.serialized_payload,
    }
  }

  pub fn data(&self) -> &SerializedPayload {
    &self.data
  }
}

impl Keyed for DDSData {
  type K = DefaultKey;
  fn get_key(&self) -> &DefaultKey {
    &self.key
  }

  fn default() -> DDSData {
    DDSData::new(DefaultKey::default(), SerializedPayload::new())
  }
}
