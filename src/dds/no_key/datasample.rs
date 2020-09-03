use crate::structure::time::Timestamp;
use crate::dds::datasample::{SampleState, ViewState, InstanceState, SampleInfo};



/// DDS spec 2.2.2.5.4
/// this is the no_key version
#[derive(Clone)]
pub struct DataSample<D> {
  pub sample_info: SampleInfo, // TODO: Can we somehow make this lazily evaluated?
  pub value: D,
}

impl<D> DataSample<D>
{
  pub fn new(source_timestamp: Timestamp, payload: D) -> DataSample<D> {

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
      },
      value: payload,
    }
  }

  //pub fn new_disposed<K>(source_timestamp: Timestamp, key: D::K) -> DataSample<D>
  // This is not implemented, because no_key samples cannot be disposed.
  // RTPS spec "8.2.9.1.3 Transition T3" seems to indicate that disposing has effect only on
  // with_key types.
}
