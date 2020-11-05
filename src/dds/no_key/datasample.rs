use crate::structure::guid::GUID;
use crate::structure::time::Timestamp;

use crate::dds::sampleinfo::*;

use crate::dds::no_key::wrappers::NoKeyWrapper;
use crate::dds::datasample::DataSample as WithKeyDataSample;

/// DDS spec 2.2.2.5.4
#[derive(PartialEq, Debug)]
pub struct DataSample<D> {
  pub(crate) sample_info: SampleInfo, // TODO: Can we somehow make this lazily evaluated?

  pub(crate) value: D,
}

impl<D> DataSample<D>
{  
  //TODO: At least rename this. This is not a proper constructor.
  pub fn new(source_timestamp: Timestamp, payload: D, writer_guid: GUID) -> DataSample<D> {
    // begin dummy placeholder values
    let sample_state = SampleState::NotRead;
    let view_state = ViewState::New;
    let instance_state = InstanceState::Alive;
    let disposed_generation_count = 0;
    let no_writers_generation_count = 0;
    let sample_rank = 0;
    let generation_rank = 0;
    let absolute_generation_rank = 0;
    // end dummy placeholder values

    DataSample {
      sample_info: SampleInfo {
        sample_state,
        view_state,
        instance_state,
        disposed_generation_count,
        no_writers_generation_count,
        sample_rank,
        generation_rank,
        absolute_generation_rank,
        source_timestamp,
        publication_handle: writer_guid,
      },
      value: payload,
    }
  }

  pub fn from_with_key(keyed: WithKeyDataSample<NoKeyWrapper<D>>) -> Option<Self> 
  {
    match keyed.value {
      Ok(kv) => Some(DataSample::<D> {
                  sample_info: keyed.sample_info,
                  value: kv.d,
                }),
      Err(_) => None,
    }
  }

  pub fn from_with_key_ref(keyed: WithKeyDataSample<&NoKeyWrapper<D>>) -> Option<DataSample<&D>> 
  {
    match keyed.value {
      Ok(ref kv) => Some(DataSample::<&D> {
                  sample_info: keyed.sample_info,
                  value: &kv.d,
                }),
      Err(_) => None,
    }
  }

} // impl
