use std::result;

// This is a specialized Result, similar to std::io::Result
pub type Result<T> = result::Result<T, Error>;

// This roughly corresponds to "Return codes" in DDS spec 2.2.1.1 Format and Conventions
#[derive(Debug)]
pub enum Error {
  // OK is not included. It is not an error.
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

pub struct LivelinessLostStatus {
  pub total: CountWithChange,
}

pub struct OfferedDeadlineMissedStatus {
  pub total: CountWithChange,
  // missing: last instance key 
}

pub struct OfferedIncompatibleQosStatus {
  pub total: CountWithChange,
  //TODO: last_policy_id: QosPolicyId_t
  //TODO: policies: QosPolicyCountSeq
}

pub struct RequestedIncompatibleQosStatus {
  pub total: CountWithChange,
  //TODO: last_policy_id: QosPolicyId_t
  //TODO: policies: QosPolicyCountSeq
}

pub struct PublicationMatchedStatus {
  pub total: CountWithChange,
  pub current: CountWithChange,
  // Missing: reference to last instance key
}

pub struct SubscriptionMatchedStatus {
  pub total: CountWithChange,
  pub current: CountWithChange,
  // Missing: reference to last instance key
}
