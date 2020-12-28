use crate::structure::guid::GUID;
use crate::structure::sequence_number::SequenceNumber;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::dds::ddsdata::DDSData;

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Copy, Clone)]
pub enum ChangeKind {
  ALIVE,
  NOT_ALIVE_DISPOSED,
  NOT_ALIVE_UNREGISTERED,
}

#[derive(Debug, Clone)]
pub struct CacheChange {
  pub kind: ChangeKind,
  pub writer_guid: GUID,
  pub sequence_number: SequenceNumber,
  pub data_value: Option<SerializedPayload>,
  pub key: u128,
  //pub inline_qos: ParameterList,

  //stps_chage_for_reader : RTPSChangeForReader
}

impl PartialEq for CacheChange {
  fn eq(&self, other: &Self) -> bool {
    let dataeq = match &self.data_value {
      Some(d1) => match &other.data_value {
        Some(d2) => d1 == d2,
        None => false,
      },
      None => other.data_value.is_none(),
    };

    self.kind == other.kind
      && self.writer_guid == other.writer_guid
      && self.sequence_number == other.sequence_number
      && dataeq
  }
}

impl CacheChange {
  pub fn new(
    kind: ChangeKind,
    writer_guid: GUID,
    sequence_number: SequenceNumber,
    data_value: Option<DDSData>,  //TODO: Why is this an Option? It seems that all callers pass Some.
  ) -> CacheChange {
    let (key, data_value) = match data_value {
      Some(d) => (d.value_key_hash, d.value()),
      None => (0, None),
    };

    CacheChange { kind, writer_guid, sequence_number, data_value, key }
  }
}

impl Default for CacheChange {
  fn default() -> Self {
    CacheChange::new(
      ChangeKind::ALIVE,
      GUID::default(),
      SequenceNumber::default(),
      None,
    )
  }
}
