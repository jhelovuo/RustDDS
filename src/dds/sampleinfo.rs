use enumflags2::{BitFlags, bitflags};

use crate::{structure::guid::GUID};
use crate::structure::time::Timestamp;

//use std::num::Zero; // unstable

/// DDS spec 2.2.2.5.4
/// "Read" indicates whether or not the corresponding data sample has already been read.
#[derive(Debug, Copy, Clone, PartialEq)]
#[bitflags]
#[repr(u32)] // DDS Spec 1.4 section 2.3.3 DCPS PSM : IDL defines these as "unsigned long", so u32
pub enum SampleState {
  Read = 0b0001,
  NotRead = 0b0010,
}

impl SampleState {
  /// Set that contains all possible states
  pub fn any() -> BitFlags<Self> {
    BitFlags::<Self>::all()
  }
}

/// DDS spec 2.2.2.5.1.8
#[derive(Debug, Copy, Clone, PartialEq)]
#[bitflags]
#[repr(u32)]
pub enum ViewState {
  ///  indicates that either this is the first time that the DataReader has ever
  /// accessed samples of that instance, or else that the DataReader has accessed previous
  /// samples of the instance, but the instance has since been reborn (i.e., become
  /// not-alive and then alive again).
  New = 0b0001,
  /// indicates that the DataReader has already accessed samples of the same
  ///instance and that the instance has not been reborn since
  NotNew = 0b0010,
}
impl ViewState {
  /// Set that contains all possible states
  pub fn any() -> BitFlags<Self> {
    BitFlags::<Self>::all()
  }
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[bitflags]
#[repr(u32)]
pub enum InstanceState {
  Alive = 0b0001,
  /// A DataWriter has actively disposed this instance
  NotAlive_Disposed = 0b0010,
  /// There are no writers alive.
  NotAlive_NoWriters = 0b0100,
}

impl InstanceState {
  /// Set that contains all possible states
  pub fn any() -> BitFlags<Self> {
    BitFlags::<Self>::all()
  }
  /// Set that contains both not_alive states.
  pub fn not_alive() -> BitFlags<Self> {
    InstanceState::NotAlive_Disposed | InstanceState::NotAlive_NoWriters
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NotAliveGenerationCounts {
  pub disposed_generation_count: i32,
  pub no_writers_generation_count: i32,
}

impl NotAliveGenerationCounts {
  /// Initial count value
  pub fn zero() -> Self {
    NotAliveGenerationCounts {
      disposed_generation_count: 0,
      no_writers_generation_count: 0,
    }
  }

  /// Marker value for "never accessed"
  pub fn sub_zero() -> Self {
    NotAliveGenerationCounts {
      disposed_generation_count: -1,
      no_writers_generation_count: -1,
    }
  }

  pub fn total(&self) -> i32 {
    self.disposed_generation_count + self.no_writers_generation_count
  }
}

/// DDS SampleInfo
#[derive(Debug, Clone, PartialEq)]
pub struct SampleInfo {
  pub sample_state: SampleState,
  pub view_state: ViewState,
  pub instance_state: InstanceState,
  // For each instance the middleware internally maintains these counts relative
  // to each DataReader. The counts capture snapshots if the corresponding
  // counters at the time the sample was received.
  pub generation_counts: NotAliveGenerationCounts,
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
  pub source_timestamp: Option<Timestamp>,

  // the publication_handle that identifies locally the DataWriter that modified
  // the instance (wrote the sample)
  pub publication_handle: GUID,
}

#[allow(clippy::new_without_default)]
impl SampleInfo {

  /* These accessor methods should not be needed, as the SampleInfo structure is likely
     not about to change too much, as it is specified by the DDS spec.

  pub fn sample_state(&self) -> SampleState {
    self.sample_state
  }

  pub fn set_sample_state(&mut self, sample_state: SampleState) {
    self.sample_state = sample_state
  }

  pub fn view_state(&self) -> ViewState {
    self.view_state
  }

  pub fn set_view_state(&mut self, view_state: ViewState) {
    self.view_state = view_state
  }

  pub fn instance_state(&self) -> InstanceState {
    self.instance_state
  }

  pub fn set_instance_state(&mut self, instance_state: InstanceState) {
    self.instance_state = instance_state
  }

  pub fn disposed_generation_count(&self) -> i32 {
    self.disposed_generation_count
  }

  pub fn no_writers_generation_count(&self) -> i32 {
    self.no_writers_generation_count
  }

  pub fn sample_rank(&self) -> i32 {
    self.sample_rank
  }

  pub fn generation_rank(&self) -> i32 {
    self.generation_rank
  }

  pub fn absolute_generation_rank(&self) -> i32 {
    self.absolute_generation_rank
  }

  pub fn source_timestamp(&self) -> Timestamp {
    self.source_timestamp
  }

  pub fn set_source_timestamp(&mut self, source_timestamp: Timestamp) {
    self.source_timestamp = source_timestamp
  }

  pub fn publication_handle(&self) -> GUID {
    self.publication_handle
  }

  pub fn set_publication_handle(&mut self, publication_handle: GUID) {
    self.publication_handle = publication_handle
  }
  */
}
