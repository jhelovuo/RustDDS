use std::result;

// This is a specialized Result, similar to std::io::Result
pub type Result<T> = result::Result<T, Error>;

// This roughly corresponds to "Return codes" in DDS spec 2.2.1.1 Format and Conventions
#[derive(Debug)]
pub enum Error {
  // OK is not included. It is not an error. Ok/Error shoudl be distinguished with the Result type.
  // Error, // unspecified, please do not use these
  BadParameter,
  Unsupported,
  // AlreadyDeleted, // we should use Rust type system to avoid these, so no need for run-time error.
  OutOfResources,
  NotEnabled,
  ImmutablePolicy, // can we check this statically?
  InconsistentPolicy,
  PreconditionNotMet,
  //Timeout,  // this is normal operation and should be encoded as Option<> or Result<>
  IllegalOperation,
  //NoData,  // this should be encoded as Option<SomeData>, not an error code
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CountWithChange {
  pub count: i32,
  pub count_change: i32,
}

pub struct InconsistentTopicStatus {
  pub total: CountWithChange,
}

pub struct SampleLostStatus {
  pub total: CountWithChange,
}

// This replaces SampleRejectedStatusKind
pub enum SampleRejectedReason {
  InstancesLimit,
  SamplesLimit,
  SamplesPerInstanceLimit,
}

pub struct SampleRejectedStatus {
  pub total: CountWithChange,
  pub last_reason: Option<SampleRejectedReason>, // None == NOT_REJECTED
                                                 // missing: last_instance_handle: instance key indicating last rejected instance
}

#[derive(Debug, Clone)]
pub enum StatusChange {
  LivelinessLostStatus {
    status: LivelinessLostStatus,
  },
  OfferedDeadlineMissedStatus {
    status: OfferedDeadlineMissedStatus,
  },
  OfferedIncompatibleQosStatus {
    status: OfferedIncompatibleQosStatus,
  },
  RequestedDeadlineMissedStatus {
    status: RequestedDeadlineMissedStatus,
  },
  RequestedIncompatibleQosStatus {
    status: RequestedIncompatibleQosStatus,
  },
  PublicationMatchedStatus {
    status: PublicationMatchedStatus,
  },
  SubscriptionMatchedStatus {
    status: SubscriptionMatchedStatus,
  },
}

#[derive(Debug,Copy, Clone)]
pub struct LivelinessLostStatus {
  pub total: CountWithChange,
}

#[derive(Debug, Copy, Clone)]
pub struct OfferedDeadlineMissedStatus {
  pub total: CountWithChange,
  // missing: last instance key
}

impl OfferedDeadlineMissedStatus {
  pub fn new() -> OfferedDeadlineMissedStatus {
    OfferedDeadlineMissedStatus {
      total: CountWithChange {
        count: 0,
        count_change: 0,
      },
    }
  }

  pub fn increase(&mut self) {
    self.total.count += 1;
    self.total.count_change += 1;
  }

  pub fn reset_change(&mut self) {
    self.total.count_change = 0;
  }
}


#[derive(Debug, Clone)]
pub struct OfferedIncompatibleQosStatus {
  pub total: CountWithChange,
  //TODO: last_policy_id: QosPolicyId_t
  //TODO: policies: QosPolicyCountSeq
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RequestedDeadlineMissedStatus {
  pub total: CountWithChange,
  // missing: last instance handle
}

impl RequestedDeadlineMissedStatus {
  pub fn new() -> RequestedDeadlineMissedStatus {
    RequestedDeadlineMissedStatus {
      total: CountWithChange {
        count: 0,
        count_change: 0,
      },
    }
  }

  pub fn increase(&mut self) {
    self.total.count += 1;
    self.total.count_change += 1;
  }

  pub fn reset_change(&mut self) {
    self.total.count_change = 0;
  }
}

#[derive(Debug,Clone)]
pub struct RequestedIncompatibleQosStatus {
  pub total: CountWithChange,
  //TODO: last_policy_id: QosPolicyId_t
  //TODO: policies: QosPolicyCountSeq
}


#[derive(Debug, Copy, Clone)]
pub struct PublicationMatchedStatus {
  pub total: CountWithChange,
  pub current: CountWithChange,
  // Missing: reference to last instance key
}


#[derive(Debug, Copy, Clone)]
pub struct SubscriptionMatchedStatus {
  pub total: CountWithChange,
  pub current: CountWithChange,
  // Missing: reference to last instance key
}
