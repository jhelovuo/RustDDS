use crate::structure::change_kind::ChangeKind;
use crate::structure::data::Data;
use crate::structure::guid::GUID;
use crate::structure::instance_handle::InstanceHandle;
use crate::structure::sequence_number::SequenceNumber;

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct CacheChange {
  pub kind: ChangeKind,
  pub writer_guid: GUID,
  pub instance_handle: InstanceHandle,
  pub sequence_number: SequenceNumber,
  pub data_value: Data,
}
