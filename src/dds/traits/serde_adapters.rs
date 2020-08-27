use serde::de::DeserializeOwned;

use crate::serialization::error::Result;

use crate::messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier;



pub trait DeserializerAdapter<D>
  where D: DeserializeOwned
{
  fn supported_encodings() -> &'static [RepresentationIdentifier]; // Which data encodings can this deserializer read?
  fn from_bytes<'de>(input_bytes: &'de [u8], encoding: RepresentationIdentifier) -> Result<D>;
}

