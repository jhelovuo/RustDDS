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
