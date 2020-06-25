use crate::structure::guid::GUID;
use crate::structure::instance_handle::InstanceHandle;
use crate::structure::sequence_number::SequenceNumber;

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub enum ChangeKind {
  ALIVE,
  NOT_ALIVE_DISPOSED,
  NOT_ALIVE_UNREGISTERED,
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct Data {}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct CacheChange {
  pub kind: ChangeKind,
  pub writer_guid: GUID,
  pub instance_handle: InstanceHandle,
  pub sequence_number: SequenceNumber,
  pub data_value: Option<Data>,
}
