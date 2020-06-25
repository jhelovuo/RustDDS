use std::result;

// This is a spcialized Result, similar to std::io::Result
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


//TODO: Since most of the struct below have (sub)structure (count , count_change), that should be made into
// a separate structure, which is then a component of these structs.

pub struct InconsistentTopicStatus {
  pub total_count: i32,
  pub total_count_change: i32,  
}

pub struct SampleLostStatus {
  pub total_count: i32,
  pub total_count_change: i32,  
}

// This replaces SampleRejectedStatusKind
pub enum SampleRejectedReason {
  InstancesLimit, SamplesLimit, SamplesPerInstanceLimit
}

pub struct SampleRejectedStatus {
  pub total_count: i32,
  pub total_count_change: i32,  
  pub last_reason: Option<SampleRejectedReason>,  // None == NOT_REJECTED
  // missing: last_instance_handle: InstanceHandle pointing to last rejected (what?)
}

pub struct LivelinessLostStatus {
  pub total_count: i32,
  pub total_count_change: i32,
}

pub struct OfferedDeadlineMissedStatus {
  pub total_count: i32,
  pub total_count_change: i32,
  // last_instance_hadle field should be here  
}




pub struct OfferedIncompatibelQosStatus {
  pub total_count: i32,
  pub total_count_change: i32,
  //TODO: last_policy_id: QosPolicyId_t
  //TODO: policies: QosPolicyCountSeq
}

pub struct RequestedIncompatibelQosStatus {
  pub total_count: i32,
  pub total_count_change: i32,
  //TODO: last_policy_id: QosPolicyId_t
  //TODO: policies: QosPolicyCountSeq
}


pub struct PublicationMatchedStatus {
  pub total_count: i32,
  pub total_count_change: i32,
  pub current_count: i32,
  pub current_count_change: i32,
  // last_instance_hadle field should be here  
}

pub struct SubscriptionMatchedStatus {
  pub total_count: i32,
  pub total_count_change: i32,
  pub current_count: i32,
  pub current_count_change: i32,
  // last_instance_hadle field should be here  
}