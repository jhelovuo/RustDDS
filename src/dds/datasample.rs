use std::sync::Arc;

use crate::dds::traits::key::*;
use crate::structure::time::Timestamp;

/// DDS spec 2.2.2.5.4
/// "Read" indicates whether or not the corresponding data sample has already been read.
#[derive(Debug)]
pub enum SampleState {
  Read,
  NotRead,
}

/// DDS spec 2.2.2.5.1.8
///
#[derive(Debug)]
pub enum ViewState {
  ///  indicates that either this is the first time that the DataReader has ever
  /// accessed samples of that instance, or else that the DataReader has accessed previous
  /// samples of the instance, but the instance has since been reborn (i.e., become
  /// not-alive and then alive again).
  New,
  /// indicates that the DataReader has already accessed samples of the same
  ///instance and that the instance has not been reborn since
  NotNew,
}

#[derive(Debug)]
pub enum InstanceState {
  Alive,
  /// A DataWriter has actively disposed this instance
  NotAlive_Disposed,
  /// There are no writers alive.
  NotAlive_NoWriters,
}

/// DDS spec 2.2.2.5.4
/// This combines SampleInfo and Data
#[derive(Debug)]
pub struct DataSample<D: Keyed> {
  pub sample_state: SampleState,
  pub view_state: ViewState,
  pub instance_state: InstanceState,
  pub disposed_generation_count: i32,
  pub no_writers_generation_count: i32,
  pub sample_rank: i32,
  pub generation_rank: i32,
  pub absolute_generation_rank: i32,
  pub source_timestamp: Timestamp,
  // instance handle
  // publication handle
  /// This ia a bit unorthodox use of Result.
  /// It replaces the use of valid_data flag, because when valid_data = false, we should
  /// not provide any data value.
  /// Now Ok(D) means valid_data = true and there is a sample.
  /// Err(D::K) means there is valid_data = false, but only a Key and instance_state has changed.
  pub value: std::result::Result<Arc<D>, D::K>,
}

impl<D: Keyed> DataSample<D> {
  pub fn new(source_timestamp: Timestamp, value: Option<D>) -> DataSample<D> {
    let sample_state = SampleState::Read;
    let view_state = ViewState::New;
    let instance_state = InstanceState::Alive;
    let disposed_generation_count = 0;
    let no_writers_generation_count = 0;
    let sample_rank = 0;
    let generation_rank = 0;
    let absolute_generation_rank = 0;
    let value = match value {
      Some(v) => v,
      None => D::default(),
    };

    DataSample {
      sample_state,
      view_state,
      instance_state,
      disposed_generation_count,
      no_writers_generation_count,
      sample_rank,
      generation_rank,
      absolute_generation_rank,
      source_timestamp,
      value: Ok(Arc::new(value)),
    }
  }
}
