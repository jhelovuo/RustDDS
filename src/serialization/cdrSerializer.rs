use serde::{ser, Serialize};
use serde::{de};
use std::fmt::{self, Display};
//use error::{Error, Result};
extern crate byteorder;
use crate::serialization::cdrSerializer::byteorder::WriteBytesExt;
use byteorder::LittleEndian;

use crate::serialization::error::Error;
use crate::serialization::error::Result;

pub struct SerializerLittleEndian {
  buffer: Vec<u8>,
  stringLogger: String,
}

// supports now only `to_little_endian_binary`.
pub fn to_little_endian_binary<T>(value: &T) -> Result<Vec<u8>>
where
  T: Serialize,
{
  let mut serializerLittleEndian = SerializerLittleEndian {
    buffer: Vec::new(),
    stringLogger: String::new(),
  };
  value.serialize(&mut serializerLittleEndian)?;
  Ok(serializerLittleEndian.buffer)
}

impl<'a> ser::Serializer for &'a mut SerializerLittleEndian {
  type Ok = ();
  // The error type when some error occurs during serialization.
  type Error = Error;

  // Associated types for keeping track of additional state while serializing
  // compound data structures like sequences and maps. In this case no
  // additional state is required beyond what is already stored in the
  // Serializer struct.
  type SerializeSeq = Self;
  type SerializeTuple = Self;
  type SerializeTupleStruct = Self;
  type SerializeTupleVariant = Self;
  type SerializeMap = Self;
  type SerializeStruct = Self;
  type SerializeStructVariant = Self;

  //Little-Endian endcoding least significant bit is first.

  //15.3.1.5 Boolean
  //  Boolean values are encoded as single octets, where TRUE is the value 1, and FALSE as 0.
  fn serialize_bool(self, v: bool) -> Result<()> {
    if v {
      self.buffer.push(1);
      self.stringLogger += "True";
    } else {
      self.buffer.push(0);
      self.stringLogger += "False";
    }

    Ok(())
  }

  //Figure 15-1 on page 15-7 illustrates the representations for OMG IDL integer data
  //types, including the following data types:
  //short
  //unsigned short
  //long
  //unsigned long
  //long long
  //unsigned long long

  fn serialize_u8(self, v: u8) -> Result<()> {
    self.buffer.push(v);
    Ok(())
  }

  fn serialize_u16(self, v: u16) -> Result<()> {
    let mut wtr = vec![];
    wtr.write_u16::<LittleEndian>(v).unwrap();
    self.buffer.push(wtr[0]);
    self.buffer.push(wtr[1]);
    Ok(())
  }

  fn serialize_u32(self, v: u32) -> Result<()> {
    let mut wtr = vec![];
    wtr.write_u32::<LittleEndian>(v).unwrap();
    self.buffer.push(wtr[0]);
    self.buffer.push(wtr[1]);
    self.buffer.push(wtr[2]);
    self.buffer.push(wtr[3]);
    Ok(())
  }

  fn serialize_u64(self, v: u64) -> Result<()> {
    let mut wtr = vec![];
    wtr.write_u64::<LittleEndian>(v).unwrap();
    self.buffer.push(wtr[0]);
    self.buffer.push(wtr[1]);
    self.buffer.push(wtr[2]);
    self.buffer.push(wtr[3]);
    self.buffer.push(wtr[4]);
    self.buffer.push(wtr[5]);
    self.buffer.push(wtr[6]);
    self.buffer.push(wtr[7]);
    Ok(())
  }

  fn serialize_i8(self, v: i8) -> Result<()> {
    let mut wtr = vec![];
    wtr.write_i8(v).unwrap();
    self.buffer.push(wtr[0]);
    Ok(())
  }

  fn serialize_i16(self, v: i16) -> Result<()> {
    let mut wtr = vec![];
    wtr.write_i16::<LittleEndian>(v).unwrap();
    self.buffer.push(wtr[0]);
    self.buffer.push(wtr[1]);
    Ok(())
  }

  fn serialize_i32(self, v: i32) -> Result<()> {
    let mut wtr = vec![];
    wtr.write_i32::<LittleEndian>(v).unwrap();
    self.buffer.push(wtr[0]);
    self.buffer.push(wtr[1]);
    self.buffer.push(wtr[2]);
    self.buffer.push(wtr[3]);
    Ok(())
  }

  fn serialize_i64(self, v: i64) -> Result<()> {
    let mut wtr = vec![];
    wtr.write_i64::<LittleEndian>(v).unwrap();
    self.buffer.push(wtr[0]);
    self.buffer.push(wtr[1]);
    self.buffer.push(wtr[2]);
    self.buffer.push(wtr[3]);
    self.buffer.push(wtr[4]);
    self.buffer.push(wtr[5]);
    self.buffer.push(wtr[6]);
    self.buffer.push(wtr[7]);
    Ok(())
  }

  // TODO FUNCTIONS AFTER THIS ARE NOT IMPLEMENTED
  fn serialize_f32(self, v: f32) -> Result<()> {
    Ok(())
  }
  fn serialize_f64(self, v: f64) -> Result<()> {
    Ok(())
  }
  fn serialize_char(self, v: char) -> Result<()> {
    Ok(())
  }
  fn serialize_str(self, v: &str) -> Result<()> {
    Ok(())
  }
  fn serialize_bytes(self, v: &[u8]) -> Result<()> {
    Ok(())
  }
  fn serialize_none(self) -> Result<()> {
    Ok(())
  }
  fn serialize_some<T>(self, _: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    Ok(())
  }

  fn serialize_unit(self) -> Result<()> {
    Ok(())
  }
  fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
    self.serialize_unit()
  }
  fn serialize_unit_variant(
    self,
    _name: &'static str,
    _variant_index: u32,
    variant: &'static str,
  ) -> Result<()> {
    self.serialize_str(variant)
  }
  fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self);
    Ok(())
  }

  fn serialize_newtype_variant<T>(
    self,
    _name: &'static str,
    _variant_index: u32,
    variant: &'static str,
    value: &T,
  ) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    Ok(())
  }

  fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
    Ok(self)
  }
  fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
    self.serialize_seq(Some(len))
  }
  fn serialize_tuple_struct(
    self,
    _name: &'static str,
    len: usize,
  ) -> Result<Self::SerializeTupleStruct> {
    self.serialize_seq(Some(len))
  }
  fn serialize_tuple_variant(
    self,
    _name: &'static str,
    _variant_index: u32,
    variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeTupleVariant> {
    Ok(self)
  }
  fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
    Ok(self)
  }
  fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
    self.serialize_map(Some(len))
  }
  fn serialize_struct_variant(
    self,
    _name: &'static str,
    _variant_index: u32,
    variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeStructVariant> {
    Ok(self)
  }
}

impl<'a> ser::SerializeSeq for &'a mut SerializerLittleEndian {
  type Ok = ();
  type Error = Error;

  fn serialize_element<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    Ok(())
  }
}

impl<'a> ser::SerializeTuple for &'a mut SerializerLittleEndian {
  type Ok = ();
  type Error = Error;

  fn serialize_element<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    Ok(())
  }
}

impl<'a> ser::SerializeTupleStruct for &'a mut SerializerLittleEndian {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }
  fn end(self) -> Result<()> {
    Ok(())
  }
}

impl<'a> ser::SerializeTupleVariant for &'a mut SerializerLittleEndian {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }
  fn end(self) -> Result<()> {
    Ok(())
  }
}

impl<'a> ser::SerializeMap for &'a mut SerializerLittleEndian {
  type Ok = ();
  type Error = Error;
  fn serialize_key<T>(&mut self, key: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    key.serialize(&mut **self)
  }

  fn serialize_value<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    Ok(())
  }
}

impl<'a> ser::SerializeStruct for &'a mut SerializerLittleEndian {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    key.serialize(&mut **self);
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    Ok(())
  }
}

impl<'a> ser::SerializeStructVariant for &'a mut SerializerLittleEndian {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    key.serialize(&mut **self);
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::serialization::cdrSerializer::to_little_endian_binary;
  use std::fs::File;
  use std::io::prelude::*;
  use serde::{Serialize, Deserialize};
  #[test]
  fn CDR_serializationTest() {
    #[derive(Serialize)]
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
    let mut file = File::create("serialization_result_from_cdr_test").unwrap();
    file.write_all(&sarjallistettu).unwrap();
  }
}
