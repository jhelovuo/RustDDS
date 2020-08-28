use std::io;

use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::serialization::error::Result;

use crate::messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier;


/// DeserializerAdapter is used to fit serde Deserializer implementations and DataReader together.
/// DataReader cannot assume a specific serialization format, so it needs to be given as a parameter.
pub trait DeserializerAdapter<D>
where
  D: DeserializeOwned,
{
  fn supported_encodings() -> &'static [RepresentationIdentifier]; // Which data encodings can this deserializer read?
  fn from_bytes<'de>(input_bytes: &'de [u8], encoding: RepresentationIdentifier) -> Result<D>;
}


pub trait SerializerAdapter<D>
  where D: Serialize
{
  fn output_encoding() -> RepresentationIdentifier; 
  fn to_writer<W: io::Write>(writer: W, value: &D) -> Result<()>;
}
