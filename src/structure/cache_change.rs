use crate::structure::change_kind::ChangeKind_t;
use crate::structure::data::Data;
use crate::structure::guid::GUID_t;
use crate::structure::instance_handle::InstanceHandle_t;
use crate::structure::sequence_number::SequenceNumber_t;

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct CacheChange {
    pub kind: ChangeKind_t,
    pub writerGuid: GUID_t,
    pub instanceHandle: InstanceHandle_t,
    pub sequenceNumber: SequenceNumber_t,
    pub data_value: Data,
}
