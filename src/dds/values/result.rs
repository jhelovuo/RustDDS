use std;
use std::result;

#[allow(unused_imports)]
use log::{debug, info, warn, trace, error};

use mio_extras::channel::{TrySendError};

/// This is a specialized Result, similar to std::io::Result
pub type Result<T> = result::Result<T, Error>;

/// This roughly corresponds to "Return codes" in DDS spec 2.2.1.1 Format and Conventions
///
/// Deviations from the DDS spec:
/// * `OK` is not included. It is not an error. Ok/Error should be distinguished with the `Result` type.
/// * `Error` is too unspecific.
/// * `AlreadyDeleted` We should use Rust type system to avoid these, so no need for run-time error.
/// * `Timeout`  This is normal operation and should be encoded as `Option` or `Result`
/// * `NoData`  This should be encoded as `Option<SomeData>`, not an error code.
#[derive(Debug)]
pub enum Error {
  /// Illegal parameter value.
  BadParameter { reason: String },
  /// Unsupported operation. Can only be returned by operations that are optional.
  Unsupported,
  /// Service ran out of the resources needed to complete the operation.
  OutOfResources,
  /// Operation invoked on an Entity that is not yet enabled.
  NotEnabled,
  /// Application attempted to modify an immutable QosPolicy.
  ImmutablePolicy, // can we check this statically?
  /// Application specified a set of policies that are not consistent with each other.
  InconsistentPolicy { reason: String },
  /// A pre-condition for the operation was not met.
  PreconditionNotMet { precondition: String },
  /// An operation was invoked on an inappropriate object or at
  /// an inappropriate time (as determined by policies set by the
  /// specification or the Service implementation). There is no
  /// precondition that could be changed to make the operation
  /// succeed.
  IllegalOperation { reason: String },

  // Our own additions to the DDS spec below:

  /// Synchronization with another thread failed because the [other thread
  /// has exited while holding a lock.](https://doc.rust-lang.org/std/sync/struct.PoisonError.html)
  /// Does not exist in the DDS spec.
  LockPoisoned,

  /// Something that should not go wrong went wrong anyway.
  /// This is usually a bug in RustDDS
  Internal { reason: String },

  Io { inner: std::io::Error },
  Serialization { reason: String },
  Discovery {reason: String},
}


impl Error {
  pub fn bad_parameter<T>(reason: &str) -> Result<T> 
  { 
    Err( Error::BadParameter{ reason: reason.to_string() }) 
  }

  pub fn precondition_not_met<T>(precondition: &str) -> Result<T> 
  { 
    Err( Error::PreconditionNotMet{ precondition: precondition.to_string() }) 
  }
  
}

#[doc(hidden)]
#[macro_export]
macro_rules! log_and_err_precondition_not_met {
  ($err_msg:literal) => (
      { error!($err_msg);
        Error::precondition_not_met($err_msg)
      }
    )
}

#[doc(hidden)]
#[macro_export]
macro_rules! log_and_err_internal {
  ($($arg:tt)*) => (
      { error!($($arg)*);
        Err( Error::Internal{ reason: format!($($arg)*) } )
      }
    )
}

#[doc(hidden)]
#[macro_export]
macro_rules! log_and_err_discovery {
  ($($arg:tt)*) => (
      { error!($($arg)*);
        Error::Message(format!($($arg)*) ) 
      }
    )
}

impl From<std::io::Error> for Error {
  fn from(e:std::io::Error) -> Error {
    Error::Io { inner: e }
  }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
  fn from(_e : std::sync::PoisonError<T>) -> Error {
    Error::LockPoisoned
  }
}

impl<T> From<TrySendError<T>> for Error 
where TrySendError<T> : std::error::Error 
{
  fn from(e : TrySendError<T>) -> Error {
    Error::Internal{reason: format!("Cannot send to internal mio channel: {:?}",e) }
  }
}

