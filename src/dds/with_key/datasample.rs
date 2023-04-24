use crate::dds::{sampleinfo::*,};
use crate::{
  dds::{traits::key::*, with_key::datawriter::WriteOptions},
  structure::{guid::GUID, sequence_number::SequenceNumber, time::Timestamp, 
    cache_change::CacheChange,},
};
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

// This structure is used to communicate just deserialized samples
// from SimpleDatareader to DataReader
#[derive(Debug, Clone)]
pub struct DeserializedCacheChange<D: Keyed> {
  pub(crate) receive_instant: Timestamp, /* 8 bytes, to be used as unique key in internal data
                                          * structures */
  pub(crate) writer_guid: GUID,               // 8 bytes
  pub(crate) sequence_number: SequenceNumber, // 8 bytes
  pub(crate) write_options: WriteOptions,     // 16 bytes

  // the data sample (or key) itself is stored here
  pub(crate) sample: Sample<D, D::K>, /* TODO: make this a Box<> for easier detaching an
                                       * reattaching to somewhere else */
}

impl<D: Keyed> DeserializedCacheChange<D> {
  pub fn new(receive_instant: Timestamp, cc: &CacheChange, deserialized: Sample<D, D::K>) -> Self {
    DeserializedCacheChange {
      receive_instant,
      writer_guid: cc.writer_guid,
      sequence_number: cc.sequence_number,
      write_options: cc.write_options.clone(),
      sample: deserialized,
    }
  }
}
