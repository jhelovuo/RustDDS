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

pub struct LivelinessLostStatus {
  total_count: i32,
  total_count_change: i32,
}

pub struct OfferedDeadlineMissedStatus {
  total_count: i32,
  total_count_change: i32,
  // last_instance_hadle field should be here  
}

pub struct OfferedIncompatibelQosStatus {
  total_count: i32,
  total_count_change: i32,
  //TODO: last_policy_id: QosPolicyId_t
  //TODO: policies: QosPolicyCountSeq
}

pub struct PublicationMatchedStatus {
  total_count: i32,
  total_count_change: i32,
  current_count: i32,
  current_count_change: i32,
  // last_instance_hadle field should be here  
}