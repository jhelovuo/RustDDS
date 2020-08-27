use serde::de::{Visitor, Error};
use super::{builtin_data_deserializer::BuiltinDataDeserializer};

impl<'de> Visitor<'de> for BuiltinDataDeserializer {
  type Value = BuiltinDataDeserializer;
  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "ParameterId")
  }

  fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(self.parse_data(v))
  }
}
