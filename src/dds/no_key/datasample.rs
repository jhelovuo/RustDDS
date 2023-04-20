use crate::dds::{
  no_key::wrappers::NoKeyWrapper,
  sampleinfo::SampleInfo,
  with_key::datasample::{DataSample as WithKeyDataSample, Sample},
};

/// A data sample and its associated [metadata](`SampleInfo`) received from a
/// NO_KEY Topic.
///
/// See DDS spec version 1.4 Section 2.2.2.5.4
///
/// Note that [`no_key::DataSample`](crate::no_key::DataSample) and
/// [`with_key::DataSample`](crate::with_key::DataSample) are two different
/// structs.
#[derive(PartialEq, Eq, Debug)]
pub struct DataSample<D> {
  pub(crate) sample_info: SampleInfo, // TODO: Can we somehow make this lazily evaluated?

  pub(crate) value: D,
}

impl<D> DataSample<D> {
  pub(crate) fn from_with_key(keyed: WithKeyDataSample<NoKeyWrapper<D>>) -> Option<Self> {
    match keyed.value {
      Sample::Value(kv) => Some(Self {
        sample_info: keyed.sample_info,
        value: kv.d,
      }),
      Sample::Dispose(_) => None,
    }
  }

  pub(crate) fn from_with_key_ref(
    keyed: WithKeyDataSample<&NoKeyWrapper<D>>,
  ) -> Option<DataSample<&D>> {
    match keyed.value {
      Sample::Value(kv) => Some(DataSample::<&D> {
        sample_info: keyed.sample_info,
        value: &kv.d,
      }),
      Sample::Dispose(_) => None,
    }
  }

  pub fn value(&self) -> &D {
    &self.value
  }

  pub fn into_value(self) -> D {
    self.value
  }

  pub fn sample_info(&self) -> &SampleInfo {
    &self.sample_info
  }

  pub fn sample_info_mut(&mut self) -> &mut SampleInfo {
    &mut self.sample_info
  }
} // impl
