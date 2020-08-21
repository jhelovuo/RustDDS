use enumflags2::BitFlags;

use crate::dds::datasample::*;

// This is used to specify which samples are to be read or taken.
// To be selected, the current state of the sample must be included in the corresponding bitflags.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ReadCondition {
  pub sample_state_mask: BitFlags<SampleState>,
  pub view_state_mask: BitFlags<ViewState>,
  pub instance_state_mask: BitFlags<InstanceState>,
  // Extension idea: Add a query string and a list of query parameters to upgrade this
  // to QueryCondition. But that would be a lot of work, especially in DataReader.
}

impl ReadCondition {
  pub fn any() -> ReadCondition {
    ReadCondition {
      sample_state_mask: SampleState::any(),
      view_state_mask: ViewState::any(),
      instance_state_mask: InstanceState::any(),
    }
  }

  pub fn not_read() -> ReadCondition {
    ReadCondition {
      sample_state_mask: SampleState::NotRead.into(),
      view_state_mask: ViewState::any(),
      instance_state_mask: InstanceState::any(),
    }
  }
}
