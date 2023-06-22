//! This module corresponds to "Return codes" in DDS spec Section "2.2.1.1
//! Format and Conventions", but the error codes are not the same.
//! In particular, a uniform error type is not used for all DDS calls, because
//! most calls can only return a subset of errors.
//! Using specialized error types makes the description of possible failures
//! more preceise.

use crate::{no_key::wrappers::NoKeyWrapper, TopicKind};

/// Error type for DDS "read" type operations.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
  /// Data received over RTPS could not be decoded.
  /// Reason field gives more details on what went wrong.
  #[error("Deserialization error: {reason}")]
  Deserialization { reason: String },

  /// DDS received a data instance dispose message via RTPS, but was asked to
  /// dispose an instance, which we do not have.
  ///
  /// If that data instance was published and last available for reading before
  /// this DataReader joined the domain (i.e. was started), then it is normal
  /// that we do not know it. Otherwise, it could be a symptom of communication
  /// or serialization error.
  #[error("Received dispose message with unknown key: {details}")]
  UnknownKey { details: String },

  /// Communication or synchronization with RTPS processing thread or Discovery
  /// thread fails. This is most likely because either thread has panicked or
  /// gotten stuck somewhere, neither of which is supposed to happen. This is
  /// typically not recoverable, except by starting a new DomainParticipant.
  #[error("Cannot communicate with background thread. It may have paniced. Details: {reason}")]
  Poisoned { reason: String },
}

/// This is a specialized Result, similar to [`std::io::Result`].
pub type ReadResult<T> = std::result::Result<T, ReadError>;

/// Error type for DDS "Write" operations.
///
/// Note: This type contains payload data type `D`. This means that `WriteError`
/// implements `Debug` only if `D` does.
#[derive(Debug, thiserror::Error)]
pub enum WriteError<D> {
  /// Data serializer (`SerializerAdapter`) erpoted an error when called.
  /// Reason field gives more details on what went wrong.
  #[error("Serialization error: {reason}")]
  Serialization { reason: String, data: D },

  /// Communication or synchronization with RTPS processing thread or Discovery
  /// thread fails. This is most likely because either thread has panicked or
  /// gotten stuck somewhere, neither of which is supposed to happen. This is
  /// typically not recoverable, except by starting a new DomainParticipant.
  #[error("Cannot communicate. Background thread may have paniced: {reason}")]
  Poisoned { reason: String, data: D },

  /// a [`std::io::Error`] occurred within RustDDS.
  #[error("std:io:Error {0}")]
  Io(#[from] std::io::Error),

  /// The operation would block, or blocked until specified timeout expired.
  #[error("Write operation timed out while blocking")]
  WouldBlock { data: D },

  /// Something that should not go wrong went wrong anyway.
  /// This is usually a bug in RustDDS
  #[error("Internal error: {reason}")]
  Internal { reason: String },
}

/// This is a specialized Result, similar to [`std::io::Result`].
pub type WriteResult<T, D> = std::result::Result<T, WriteError<D>>;

pub(crate) fn unwrap_nokey<D>(nokey: WriteError<NoKeyWrapper<D>>) -> WriteError<D> {
  match nokey {
    WriteError::Serialization { reason, data } => WriteError::Serialization {
      reason,
      data: data.d,
    },
    WriteError::Poisoned { reason, data } => WriteError::Poisoned {
      reason,
      data: data.d,
    },
    WriteError::WouldBlock { data } => WriteError::WouldBlock { data: data.d },
    WriteError::Internal { reason } => WriteError::Internal { reason },
    WriteError::Io(io) => WriteError::Io(io),
  }
}

/// Error type for object cration operations.
#[derive(Debug, thiserror::Error)]
pub enum CreateError {
  #[error("Object creation failed, because necessary resource has been dropped: {reason}")]
  ResourceDropped { reason: String },

  #[error("Cannot communicate. Background thread may have paniced: {reason}")]
  Poisoned { reason: String },

  #[error("std:io:Error {0}")]
  Io(#[from] std::io::Error),

  #[error("Wrong Topic kind. Expected {0}")]
  TopicKind(TopicKind),

  /// Something that should not go wrong went wrong anyway.
  /// This is usually a bug in RustDDS
  #[error("Internal error: {reason}")]
  Internal { reason: String },

  #[error("Invalid call parameter: {reason}")]
  BadParameter { reason: String },

  #[error("Resource allocation failed: {reason}")]
  OutOfResources { reason: String },
}

/// This is a specialized Result, similar to [`std::io::Result`].
pub type CreateResult<T> = std::result::Result<T, CreateError>;

#[doc(hidden)]
#[macro_export]
macro_rules! create_error_poisoned {
  ($($arg:tt)*) => (
      { log::error!($($arg)*);
        Err( CreateError::Poisoned{ reason: format!($($arg)*) } )
      }
    )
}

#[doc(hidden)]
#[macro_export]
macro_rules! create_error_dropped {
  ($($arg:tt)*) => (
      { log::error!($($arg)*);
        Err( CreateError::ResourceDropped{ reason: format!($($arg)*) } )
      }
    )
}

#[doc(hidden)]
#[macro_export]
macro_rules! create_error_out_of_resources {
  ($($arg:tt)*) => (
      { log::error!($($arg)*);
        Err( CreateError::OutOfResources{ reason: format!($($arg)*) } )
      }
    )
}

#[derive(Debug, thiserror::Error)]
pub enum WaitError {
  #[error("Waiting timed out")]
  Timeout,
}

pub type WaitResult<T> = std::result::Result<T, WaitError>;

#[derive(Debug, thiserror::Error)]
pub enum QosError {
  #[error("Parameter value or combination of values was bad. Details: {details}")]
  BadParameter { details: String },
}
