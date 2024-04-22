use std::fmt::Display;

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

// This is a bare-bones implementation. A real library would provide additional
// information in its error type, for example the line and column at which the
// error occurred, the byte offset into the input, or the current key being
// processed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  // One or more variants that can be created by data structures through the
  // `ser::Error` and `de::Error` traits. For example the Serialize impl for
  // Mutex<T> might return an error because the mutex is poisoned, or the
  // Deserialize impl for a struct may return an error because a required
  // field is missing.
  /// Wrapper for error string
  #[error("{0}")]
  Message(String),

  /// Wrapper for `std::io::Error`
  #[error("io::Error: {0}")]
  Io(#[from] std::io::Error),

  /// CDR is not self-describing format, cannot deserialize \'Any\' type
  #[error("CDR is not self-describing format, cannot deserialize \'Any\' type: {0}")]
  NotSelfDescribingFormat(String),

  /// Serialization must know sequence length before serialization.
  #[error("CDR serialization requires sequence length to be specified at the start.")]
  SequenceLengthUnknown,

  /// Unexpected end of input
  #[error("unexpected end of input")]
  Eof,

  /// Bad encoding of Boolean value
  #[error("Expected 0 or 1 as Boolean, got: {0}")]
  BadBoolean(u8),

  /// Bad Unicode codepoint
  #[error("Bad Unicode character code: {0}")]
  BadChar(u32), // invalid Unicode codepoint

  /// Bad discriminant (variant tag) in `Option`
  #[error("Option value must have discriminant 0 or 1, read: {0}")]
  BadOption(u32), // Option variant tag (discriminant) is not 0 or 1

  // String was not valid UTF-8
  #[error("UTF-8 error: {0}")]
  BadUTF8(std::str::Utf8Error),
}

impl ser::Error for Error {
  fn custom<T: Display>(msg: T) -> Self {
    Self::Message(msg.to_string())
  }
}

impl de::Error for Error {
  fn custom<T: Display>(msg: T) -> Self {
    Self::Message(msg.to_string())
  }
}
