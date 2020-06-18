use crate::structure::change_kind::ChangeKind_t;
use crate::structure::cache_change::CacheChange;
use crate::structure::data::Data;
use crate::structure::instance_handle::InstanceHandle_t;
use crate::structure::sequence_number::SequenceNumber_t;

pub struct WriterAttributes {
  pub push_mode: bool,
  // pub heartbeatPeriod: Duration_t,
  // pub nackResponseDelay: Duration_t,
  // pub nackSuppressionDuration: Duration_t,
  pub lastChangeSequenceNumber: SequenceNumber_t,
}

pub trait Writer {
  fn as_writer(&self) -> &WriterAttributes;
  fn new_change(kind: ChangeKind_t, data: Data, handle: InstanceHandle_t) -> CacheChange;
}
