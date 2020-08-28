use serde::{Deserializer, de::DeserializeOwned};
use std::marker::PhantomData;
use byteorder::{LittleEndian, ByteOrder, BigEndian};

use crate::serialization::error::Error;
use crate::{
  messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier,
  serialization::error::Result,
};

use crate::dds::traits::serde_adapters::DeserializerAdapter;

pub struct PlCdrDeserializerAdapter<D> {
  phantom: PhantomData<D>,
}

const repr_ids: [RepresentationIdentifier; 2] = [
  RepresentationIdentifier::PL_CDR_BE,
  RepresentationIdentifier::PL_CDR_LE,
];

impl<D> DeserializerAdapter<D> for PlCdrDeserializerAdapter<D>
where
  D: DeserializeOwned,
{
  fn supported_encodings() -> &'static [RepresentationIdentifier] {
    &repr_ids
  }

  fn from_bytes<'de>(input_bytes: &'de [u8], encoding: RepresentationIdentifier) -> Result<D> {
    match encoding {
      RepresentationIdentifier::PL_CDR_LE => {
        PlCdrDeserializer::<LittleEndian>::from_bytes::<D>(input_bytes)
      }
      RepresentationIdentifier::PL_CDR_BE => {
        PlCdrDeserializer::<BigEndian>::from_bytes::<D>(input_bytes)
      }
      repr_id => Err(Error::Message(format!(
        "Unknown representation identifier {}",
        u16::from(repr_id)
      ))),
    }
  }
}

pub struct PlCdrDeserializer<'de, BO> {
  phantom: PhantomData<BO>,
  input: &'de [u8],
}

impl<'de, BO> PlCdrDeserializer<'de, BO>
where
  BO: ByteOrder,
{
  pub fn new(s: &'de [u8]) -> PlCdrDeserializer<'de, BO> {
    PlCdrDeserializer::<BO> {
      phantom: PhantomData,
      input: s,
    }
  }

  pub fn from_bytes<'a, T: DeserializeOwned>(s: &'a [u8]) -> Result<T> {
    let deserializer: PlCdrDeserializer<BO> = PlCdrDeserializer::new(s);
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
