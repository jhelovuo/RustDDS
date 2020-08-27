use serde::{Deserializer, de::DeserializeOwned};
use std::marker::PhantomData;
use byteorder::ByteOrder;

use crate::serialization::error::Error;
use crate::serialization::error::Result;

pub struct PlCdrDeserializer<'de, BO> {
  phantom: PhantomData<BO>,
  input: &'de [u8],
}

impl<'de, BO> PlCdrDeserializer<'de, BO>
where
  BO: ByteOrder,
{
  pub fn new<B: ByteOrder>(s: &'de [u8]) -> PlCdrDeserializer<'de, B> {
    PlCdrDeserializer {
      phantom: PhantomData,
      input: s,
    }
  }

  pub fn from_bytes<'a, T: DeserializeOwned, B: ByteOrder>(s: &'a [u8]) -> Result<T> {
    let deserializer: PlCdrDeserializer<B> = PlCdrDeserializer::<B>::new(s);
    let t: T = T::deserialize(deserializer)?;
    Ok(t)
  }
}

impl<'de, BO> Deserializer<'de> for PlCdrDeserializer<'de, BO> {
  type Error = Error;

  fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    visitor.visit_bytes(self.input)
  }

  fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_tuple_struct<V>(
    self,
    _name: &'static str,
    _len: usize,
    _visitor: V,
  ) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_struct<V>(
    self,
    _name: &'static str,
    _fields: &'static [&'static str],
    _visitor: V,
  ) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_enum<V>(
    self,
    _name: &'static str,
    _variants: &'static [&'static str],
    _visitor: V,
  ) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: serde::de::Visitor<'de>,
  {
    unimplemented!()
  }
}
