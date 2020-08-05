use std::sync::Arc;

use crate::dds::traits::key::*;
use crate::structure::time::Timestamp;
use crate::dds::traits::datasample_trait::DataSampleTrait;

/// DDS spec 2.2.2.5.4
/// "Read" indicates whether or not the corresponding data sample has already been read.
#[derive(Debug, Clone)]
pub enum SampleState {
  Read,
  NotRead,
}

/// DDS spec 2.2.2.5.1.8
///
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum InstanceState {
  Alive,
  /// A DataWriter has actively disposed this instance
  NotAlive_Disposed,
  /// There are no writers alive.
  NotAlive_NoWriters,
}

/// DDS spec 2.2.2.5.4
/// This combines SampleInfo and Data
#[derive(Clone)]
pub struct DataSample {
  pub sample_state: SampleState,
  pub view_state: ViewState,
  pub instance_state: InstanceState,
  // For each instance the middleware internally maintains these counts relative
  // to each DataReader. The counts capture snapshots if the corresponding
  // counters at the time the sample was received.
  pub disposed_generation_count: i32,
  pub no_writers_generation_count: i32,
  // The ranks are are computed based solely on the actual samples in the
  // ordered collection returned by the read or take.
  // The sample_rank indicates the number of samples of the same instance that
  // follow the current one in the collection.
  pub sample_rank: i32,
  // The generation_rank indicates the difference in generations between the
  // samples S and the Most Recent Sample of the same instance that appears In
  // the returned Collection (MRSIC). It counts the number of times the instance
  // transitioned from not-alive to alive in the time from the reception of the
  // S to the  reception of MRSIC. The generation rank is computed with:
  // generation_rank =
  //(MRSIC.disposed_generation_count + MRSIC.no_writers_generation_count)
  //- (S.disposed_generation_count + S.no_writers_generation_count)
  pub generation_rank: i32,
  // The absolute_generation_rank indicates the difference in "generations"
  // between sample S and the Most Recent Sample of the instance that the
  // middlware has received (MRS). It counts the number of times the instance
  // transitioned from not-alive to alive in the time from the reception of the
  // S to the time when the read or take was called. absolute_generation_rank =
  //(MRS.disposed_generation_count + MRS.no_writers_generation_count)
  //- (S.disposed_generation_count + S.no_writers_generation_count)
  pub absolute_generation_rank: i32,
  pub source_timestamp: Timestamp,
  // instance handle
  // publication handle
  /// This ia a bit unorthodox use of Result.
  /// It replaces the use of valid_data flag, because when valid_data = false, we should
  /// not provide any data value.
  /// Now Ok(D) means valid_data = true and there is a sample.
  /// Err(D::K) means there is valid_data = false, but only a Key and instance_state has changed.
  // TODO: think value again
  pub value: std::result::Result<Arc<Box<dyn DataSampleTrait>>, Box<dyn Key>>,
}

impl DataSample {
  pub fn new<D: DataSampleTrait + 'static>(source_timestamp: Timestamp, value: D) -> DataSample {
    let sample_state = SampleState::Read;
    let view_state = ViewState::New;
    let instance_state = InstanceState::Alive;
    let disposed_generation_count = 0;
    let no_writers_generation_count = 0;
    let sample_rank = 0;
    let generation_rank = 0;
    let absolute_generation_rank = 0;
    let bx: Box<dyn DataSampleTrait> = Box::new(value);
    let value = Ok(Arc::new(bx));

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
      value,
    }
  }

  pub fn new_with_arc(
    source_timestamp: Timestamp,
    arc: Arc<Box<dyn DataSampleTrait>>,
  ) -> DataSample {
    let sample_state = SampleState::Read;
    let view_state = ViewState::New;
    let instance_state = InstanceState::Alive;
    let disposed_generation_count = 0;
    let no_writers_generation_count = 0;
    let sample_rank = 0;
    let generation_rank = 0;
    let absolute_generation_rank = 0;

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
      value: Ok(arc),
    }
  }

  pub fn get_value_with_type<D: DataSampleTrait>(&self) -> Option<D> {
    let dcval = match &self.value {
      Ok(val) => val,
      _ => return None,
    };

    let cpbox = dcval.box_clone();
    let dcval = cpbox.downcast::<D>();
    let dcval = match dcval {
      Ok(val) => val,
      _ => return None,
    };

    Some(*dcval)
  }
}
