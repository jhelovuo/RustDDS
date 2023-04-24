use crate::dds::{sampleinfo::*, traits::key::*};

/// A data sample received from a WITH_KEY Topic without the associated
/// metadata.
///
/// Replaces the use of `valid_data` flag in SampleInfo of DataSample from the
/// DDS spec.
///
/// Implements the methods `value`, `map_value` and `map_dispose` that
/// correspond to methods of [`Result`](std::result::Result), which had been
/// previously used for this purpose.
#[derive(Clone, PartialEq, Debug)]
pub enum Sample<D, K> {
  Value(D),
  Dispose(K),
}

impl<D, K> Sample<D, K> {
  pub fn value(self) -> Option<D> {
    match self {
      Sample::Value(d) => Some(d),
      Sample::Dispose(_) => None,
    }
  }

  pub fn map_value<D2, F: FnOnce(D) -> D2>(self, op: F) -> Sample<D2, K> {
    match self {
      Sample::Value(d) => Sample::Value(op(d)),
      Sample::Dispose(k) => Sample::Dispose(k),
    }
  }

  pub fn map_dispose<K2, F: FnOnce(K) -> K2>(self, op: F) -> Sample<D, K2> {
    match self {
      Sample::Value(d) => Sample::Value(d),
      Sample::Dispose(k) => Sample::Dispose(op(k)),
    }
  }
}

/// A data sample and its associated [metadata](`SampleInfo`) received from a
/// WITH_KEY Topic.
///
/// Note that [`no_key::DataSample`](crate::no_key::DataSample) and
/// [`with_key::DataSample`](crate::with_key::DataSample) are two different
/// structs.
///
/// We are using [`Sample`](crate::with_key::Sample) to replace the `valid_data`
/// flag from the DDS spec, because when `valid_data = false`, the application
/// should not be able to access any data.
///
/// Sample usage:
/// * `Sample::Value(d)` means `valid_data == true` and there is a sample `d`.
/// * `Sample::Dispose(k)` means `valid_data == false`, no sample exists, but
///   only a Key `k` and instance_state has changed.
///
/// See also DDS spec v1.4 Section 2.2.2.5.4.
#[derive(PartialEq, Debug)]
pub struct DataSample<D: Keyed> {
  pub(crate) sample_info: SampleInfo, // TODO: Can we somehow make this lazily evaluated?

  pub(crate) value: Sample<D, D::K>,
}

impl<D> DataSample<D>
where
  D: Keyed,
{
  pub(crate) fn new(sample_info: SampleInfo, value: Sample<D, D::K>) -> Self {
    Self { sample_info, value }
  }

  // convenience shorthand to get the key directly, without digging out the
  // "value"
  pub fn key(&self) -> D::K
  where
    <D as Keyed>::K: Key,
  {
    match &self.value {
      Sample::Value(d) => d.key(),
      Sample::Dispose(k) => k.clone(),
    }
  } // fn

  pub fn value(&self) -> &Sample<D, D::K> {
    &self.value
  }

  pub fn into_value(self) -> Sample<D, D::K> {
    self.value
  }

  pub fn sample_info(&self) -> &SampleInfo {
    &self.sample_info
  }

  pub fn sample_info_mut(&mut self) -> &mut SampleInfo {
    &mut self.sample_info
  }
} // impl
