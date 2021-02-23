use crate::{dds::traits::key::*, structure::guid::GUID};
use crate::structure::time::Timestamp;
use crate::dds::sampleinfo::*;

//use super::{interfaces::{IDataSample, IDataSampleConvert, IKeyedDataSample, IKeyedDataSampleConvert}, no_key::wrappers::NoKeyWrapper};

/// DDS spec 2.2.2.5.4
///
/// Note that no_key::DataSample and with_key::DataSample are two different but similar structs.
///
/// We are making a bit unorthodox use of `Result`:
/// It replaces the use of valid_data flag, because when valid_data = false, we should
/// not provide any data value.
/// Now 
/// * `Ok(D)` means valid_data = true and there is a sample.
/// * `Err(D::K)` means valid_data = false, no sample exists, but only a Key and instance_state has changed.

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

  pub(crate) fn new_disposed<K>(
    source_timestamp: Timestamp,
    key: D::K,
    writer_guid: GUID,
  ) -> DataSample<D>
  where
    <D as Keyed>::K: Key,
  {
    // begin dummy placeholder values
    let sample_state = SampleState::NotRead;
    let view_state = ViewState::New;
    let instance_state = InstanceState::NotAlive_Disposed;
    let sample_rank = 0;
    let generation_rank = 0;
    let absolute_generation_rank = 0;
    // end dummy placeholder values

    DataSample {
      sample_info: SampleInfo {
        sample_state,
        view_state,
        instance_state,
        generation_counts: NotAliveGenerationCounts::zero(),
        sample_rank,
        generation_rank,
        absolute_generation_rank,
        source_timestamp: Some(source_timestamp),
        publication_handle: writer_guid,
      },
      value: Err(key),
    }
  } // fn

  // convenience shorthand to get the key directly, without digging out the "value"
  pub fn get_key(&self) -> D::K
  where
    <D as Keyed>::K: Key,
  {
    match &self.value {
      Ok(d) => d.get_key(),
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
