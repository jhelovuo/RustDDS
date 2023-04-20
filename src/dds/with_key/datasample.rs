use crate::dds::{sampleinfo::*, traits::key::*};

//TODO: Fix documentation
#[derive(Clone, PartialEq, Debug)]
pub enum Sample<D, K> {
  Value(D),
  Dispose(K),
}

impl<D, K> Sample<D, K> {
  //From the Result<D,K> implementations
  pub fn value(self) -> Option<D> {
    match self {
      Sample::Value(d) => Some(d),
      Sample::Dispose(_) => None,
    }
  }

  //From the Result<D,K> implementations
  pub fn map_value<D2, F: FnOnce(D) -> D2>(self, op: F) -> Sample<D2, K> {
    match self {
      Sample::Value(d) => Sample::Value(op(d)),
      Sample::Dispose(k) => Sample::Dispose(k),
    }
  }

  //From the Result<D,K> implementations
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
/// We are making a bit unorthodox use of [`Result`](std::result::Result) here.
/// It replaces the use of `valid_data` flag from the DDS spec, because when
/// `valid_data = false`, the application should not be able to access any data.
///
/// Result usage:
/// * `Ok(d)` means `valid_data == true` and there is a sample `d`.
/// * `Err(k)` means `valid_data == false`, no sample exists, but only a Key `k`
///   and instance_state has changed.
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
