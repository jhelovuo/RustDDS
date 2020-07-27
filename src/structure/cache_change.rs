use crate::structure::guid::GUID;
use crate::structure::instance_handle::InstanceHandle;
use crate::structure::sequence_number::SequenceNumber;
use crate::messages::submessages::data::Data;

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Clone)]
pub enum ChangeKind {
  ALIVE,
  NOT_ALIVE_DISPOSED,
  NOT_ALIVE_UNREGISTERED,
}

#[derive(Debug, PartialEq, Clone)]
pub struct CacheChange {
  pub kind: ChangeKind,
  pub writer_guid: GUID,
  pub instance_handle: InstanceHandle,
  pub sequence_number: SequenceNumber,
  pub data_value: Option<Data>,
  //pub inline_qos: ParameterList,
}

impl CacheChange {
  pub fn new(
    writer_guid: GUID,
    sequence_number: SequenceNumber,
    data_value: Option<Data>,
  ) -> CacheChange {
    CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid,
      instance_handle: InstanceHandle::default(),
      sequence_number,
      data_value,
      //inline_qos: ParameterList::new(),
    }
  }
}

impl Default for CacheChange {
  fn default() -> Self {
    CacheChange::new(
      GUID::default(),
      SequenceNumber::default(),
      Some(Data::default()),
    )
  }
}
