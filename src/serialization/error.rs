use std;
use std::fmt::{self, Display};
use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

// This is a bare-bones implementation. A real library would provide additional
// information in its error type, for example the line and column at which the
// error occurred, the byte offset into the input, or the current key being
// processed.
#[derive(Debug)]
pub enum Error {
  // One or more variants that can be created by data structures through the
  // `ser::Error` and `de::Error` traits. For example the Serialize impl for
  // Mutex<T> might return an error because the mutex is poisoned, or the
  // Deserialize impl for a struct may return an error because a required
  // field is missing.
  Message(String),
  IOError(std::io::Error),
  SequenceLengthUnknown,
  // Zero or more variants that can be created directly by the Serializer and
  // Deserializer without going through `ser::Error` and `de::Error`.
  Eof,
  BadBoolean(u8),
  BadString(std::str::Utf8Error), // was not valid UTF-8
  BadChar(u32), // invalid Unicode codepoint
  //TODO
  /*
  Syntax,
  ExpectedBoolean,
  ExpectedInteger,
  ExpectedString,
  ExpectedNull,
  ExpectedArray,
  ExpectedArrayComma,
  ExpectedArrayEnd,
  ExpectedMap,
  ExpectedMapColon,
  ExpectedMapComma,
  ExpectedMapEnd,
  ExpectedEnum,
  */
  TrailingCharacters(Vec<u8>),
  
}

impl ser::Error for Error {
  fn custom<T: Display>(msg: T) -> Self {
    Error::Message(msg.to_string())
  }
}

impl de::Error for Error {
  fn custom<T: Display>(msg: T) -> Self {
    Error::Message(msg.to_string())
  }
}

impl Display for Error {
  fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Error::Message(msg) => formatter.write_str(msg),
      Error::Eof => formatter.write_str("unexpected end of input"),
      Error::IOError(e) => formatter.write_fmt(format_args!("io::Error: {:?}",e)),
      Error::SequenceLengthUnknown => 
        formatter.write_str("CDR serialization requires sequence length to be specified at the start."),
      Error::BadChar(e) => formatter.write_fmt(format_args!("Bad Unicode character code: {:?}",e)),
      Error::BadBoolean(e) => formatter.write_fmt(format_args!("Expected 0 or 1 as Boolean, got: {:?}",e)),
      Error::TrailingCharacters(vec) => 
        formatter.write_fmt(format_args!("Trailing garbage, {:?} bytes",vec.len())),
      Error::BadString( utf_err) => 
        formatter.write_fmt(format_args!("UTF-8 error: {:?}", utf_err)),
      /* and so forth */
    }
  }
}

impl From<std::io::Error> for Error {
  fn from(ioerr: std::io::Error) -> Error {
    Error::IOError(ioerr)
  }
}

impl std::error::Error for Error {}
