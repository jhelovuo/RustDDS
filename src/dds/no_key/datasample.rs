use crate::{
  dds::{
    no_key::wrappers::NoKeyWrapper, sampleinfo::*,
    with_key::datasample::DataSample as WithKeyDataSample,
  },
  structure::{guid::GUID, time::Timestamp},
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
  //TODO: At least rename this. This is not a proper constructor.
  pub(crate) fn new_deprecated(
    source_timestamp: Timestamp,
    payload: D,
    writer_guid: GUID,
  ) -> DataSample<D> {
    // begin dummy placeholder values
    let sample_state = SampleState::NotRead;
    let view_state = ViewState::New;
    let instance_state = InstanceState::Alive;
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
      value: payload,
    }
  }

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
