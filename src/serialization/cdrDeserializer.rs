use std::ops::{AddAssign, MulAssign, Neg};

use serde::Deserialize;
use serde::de::{
  self, DeserializeSeed, EnumAccess, /*IntoDeserializer, MapAccess, SeqAccess,*/  VariantAccess, Visitor,
};
use crate::serialization::error::Error;
use crate::serialization::error::Result;
//use serde::Deserializer;

pub struct DeserializerLittleEndian<'de> {
  input: &'de Vec<u8>,
}

impl<'de> DeserializerLittleEndian<'de> {
  pub fn deserialize_from_little_endian(input: &'de Vec<u8>) -> Self {
    DeserializerLittleEndian { input }
  }

  //fn remove_first_byte_from_input(&mut self){
  //     self.input.remove(0);
  //}
}

pub fn deserialize_from_little_endian<'a, T>(s: &'a Vec<u8>) -> Result<T>
where
  T: Deserialize<'a>,
{
  let mut deserializer = DeserializerLittleEndian::deserialize_from_little_endian(s);
  let t = T::deserialize(&mut deserializer)?;
  if deserializer.input.is_empty() {
    Ok(t)
  } else {
    unimplemented!()
  }
}

impl<'de> DeserializerLittleEndian<'de> {
  // Look at the first byte in the input without consuming it.
  fn peek_byte(&mut self) -> Result<&u8> {
    self.input.first().ok_or(Error::Eof)
  }

  // Consume the first byte in the input.
  //fn next_byte(&mut self) -> Result<u8> {
  //    let by = self.input[0];
  //    self.remove_first_byte_from_input();
  //    //self.input = &self.input[1..].to_vec();
  //    Ok(by)
  //}

  fn parse_bool<T>(&mut self) -> Result<T> {
    unimplemented!()
  }

  fn parse_unsigned<T>(&mut self) -> Result<T>
  where
    T: AddAssign<T> + MulAssign<T> + From<u8>,
  {
    unimplemented!()
  }

  fn parse_signed<T>(&mut self) -> Result<T>
  where
    T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8>,
  {
    unimplemented!()
  }

  fn parse_string(&mut self) -> Result<&'de str> {
    unimplemented!()
  }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut DeserializerLittleEndian<'de> {
  type Error = Error;

  fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_bool(self.parse_bool()?)
  }

  fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_i8(self.parse_signed()?)
  }

  fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_i16(self.parse_signed()?)
  }

  fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_i32(self.parse_signed()?)
  }

  fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_i64(self.parse_signed()?)
  }

  fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_u8(self.parse_unsigned()?)
  }

  fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_u16(self.parse_unsigned()?)
  }

  fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_u32(self.parse_unsigned()?)
  }

  fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_u64(self.parse_unsigned()?)
  }

  fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_borrowed_str(self.parse_string()?)
  }

  fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_str(visitor)
  }

  fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_unit(visitor)
  }

  fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_newtype_struct(self)
  }

  fn deserialize_seq<V>(/*mut*/ self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_seq(visitor)
  }

  fn deserialize_tuple_struct<V>(
    self,
    _name: &'static str,
    _len: usize,
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_seq(visitor)
  }

  fn deserialize_map<V>(/*mut*/ self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_struct<V>(
    self,
    _name: &'static str,
    _fields: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_map(visitor)
  }

  fn deserialize_enum<V>(
    self,
    _name: &'static str,
    _variants: &'static [&'static str],
    _visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_str(visitor)
  }

  fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_any(visitor)
  }
}

struct Enum<'a, 'de: 'a> {
  de: &'a mut DeserializerLittleEndian<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
  fn new(de: &'a mut DeserializerLittleEndian<'de>) -> Self {
    Enum { de }
  }
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
  type Error = Error;
  type Variant = Self;

  fn variant_seed<V>(self, _seed: V) -> Result<(V::Value, Self::Variant)>
  where
    V: DeserializeSeed<'de>,
  {
    unimplemented!()
  }
}

impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
  type Error = Error;

  fn unit_variant(self) -> Result<()> {
    unimplemented!()
  }

  fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
  where
    T: DeserializeSeed<'de>,
  {
    seed.deserialize(self.de)
  }

  fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    de::Deserializer::deserialize_seq(self.de, visitor)
  }

  fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    de::Deserializer::deserialize_map(self.de, visitor)
  }
}

#[cfg(test)]
mod tests {
  use crate::serialization::cdrSerializer::to_little_endian_binary;
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
  use serde::{Serialize, Deserialize};

  #[test]
  fn CDR_DeserializationTest() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct OmaTyyppi {
      firstValue: u8,
      secondvalue: i8,
      thirdValue: i32,
      fourthValue: u64,
    }

    let mikkiHiiri = OmaTyyppi {
      firstValue: 1,
      secondvalue: -3,
      thirdValue: -5000,
      fourthValue: 90909099999999,
    };

    let sarjallistettu = to_little_endian_binary(&mikkiHiiri).unwrap();
    let rakennettu: OmaTyyppi = deserialize_from_little_endian(&sarjallistettu).unwrap();
    assert_eq!(rakennettu, mikkiHiiri)
  }
  #[test]
  fn CDR_DeserializationTest_u8() {
    let numberU8: u8 = 1;
    let serializedNumberU8 = to_little_endian_binary(&numberU8).unwrap();
    let deSerializedNmberU8 = deserialize_from_little_endian(&serializedNumberU8).unwrap();
    assert_eq!(numberU8, deSerializedNmberU8)
  }
}
