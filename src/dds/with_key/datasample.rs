use crate::dds::{sampleinfo::*, traits::key::*};

//use super::{interfaces::{IDataSample, IDataSampleConvert, IKeyedDataSample,
// IKeyedDataSampleConvert}, no_key::wrappers::NoKeyWrapper};

/// DDS spec 2.2.2.5.4
///
/// Note that no_key::DataSample and with_key::DataSample are two different but
/// similar structs.
///
/// We are making a bit unorthodox use of `Result`:
/// It replaces the use of valid_data flag, because when valid_data = false, we
/// should not provide any data value.
/// Now
/// * `Ok(D)` means valid_data = true and there is a sample.
/// * `Err(D::K)` means valid_data = false, no sample exists, but only a Key and
///   instance_state has changed.

#[derive(PartialEq, Debug)]
pub struct DataSample<D: Keyed> {
  pub(crate) sample_info: SampleInfo, // TODO: Can we somehow make this lazily evaluated?

  pub(crate) value: std::result::Result<D, D::K>,
}

impl<D> DataSample<D>
where
  D: Keyed,
{
  pub(crate) fn new(sample_info: SampleInfo, value: std::result::Result<D, D::K>) -> Self {
    DataSample { sample_info, value }
  }

  // convenience shorthand to get the key directly, without digging out the
  // "value"
  pub fn key(&self) -> D::K
  where
    <D as Keyed>::K: Key,
  {
    match &self.value {
      Ok(d) => d.key(),
      Err(k) => k.clone(),
    }
  } // fn

  pub fn value(&self) -> &Result<D, D::K> {
    &self.value
  }

  pub fn into_value(self) -> Result<D, D::K> {
    self.value
  }

  pub fn sample_info(&self) -> &SampleInfo {
    &self.sample_info
  }

  pub fn sample_info_mut(&mut self) -> &mut SampleInfo {
    &mut self.sample_info
  }
} // impl
