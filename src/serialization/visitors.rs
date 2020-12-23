use serde::{
  de::{Visitor, Error},
};
use super::{
  builtin_data_deserializer::BuiltinDataDeserializer, 
};

use crate::{messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier};

impl<'de> Visitor<'de> for BuiltinDataDeserializer {
  type Value = BuiltinDataDeserializer;
  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "ParameterId")
  }

  fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
  where
    E: Error,
  {
    match RepresentationIdentifier::from_bytes(&v[..2]) {
      Ok(RepresentationIdentifier::PL_CDR_LE) => Ok(self.parse_data_little_endian(&v[2..])),
      Ok(RepresentationIdentifier::PL_CDR_BE) => Ok(self.parse_data_big_endian(&v[2..])),
      _ => Err(E::missing_field("representation identifier")),
    }
  }
}
