use crate::{
  dds::{
    no_key::wrappers::NoKeyWrapper, sampleinfo::*,
    with_key::datasample::DataSample as WithKeyDataSample,
  },
};

/// DDS spec 2.2.2.5.4
///
/// Note that no_key::DataSample and with_key::DataSample are two different but
/// similar structs.
#[derive(PartialEq, Debug)]
pub struct DataSample<D> {
  pub(crate) sample_info: SampleInfo, // TODO: Can we somehow make this lazily evaluated?

  pub(crate) value: D,
}

impl<D> DataSample<D> {

  pub(crate) fn from_with_key(keyed: WithKeyDataSample<NoKeyWrapper<D>>) -> Option<Self> {
    match keyed.value {
      Ok(kv) => Some(DataSample::<D> {
        sample_info: keyed.sample_info,
        value: kv.d,
      }),
      Err(_) => None,
    }
  }

  pub(crate) fn from_with_key_ref(
    keyed: WithKeyDataSample<&NoKeyWrapper<D>>,
  ) -> Option<DataSample<&D>> {
    match keyed.value {
      Ok(kv) => Some(DataSample::<&D> {
        sample_info: keyed.sample_info,
        value: &kv.d,
      }),
      Err(_) => None,
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
