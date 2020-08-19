use serde::{ser, Serialize};
//use serde::{de};
//use std::fmt::{self, Display};
//use error::{Error, Result};
extern crate byteorder;
use crate::serialization::cdrSerializer::byteorder::WriteBytesExt;
use byteorder::LittleEndian;
use byteorder::BigEndian;

use crate::serialization::error::Error;
use crate::serialization::error::Result;

#[derive(PartialEq)]
pub enum Endianess {
  LittleEndian,
  BigEndian,
}

pub struct CDR_serializer {
  buffer: Vec<u8>,
  serializationEndianess: Endianess,
}

impl CDR_serializer {
  pub fn new(endianess: Endianess) -> CDR_serializer {
    CDR_serializer {
      buffer: Vec::new(),
      serializationEndianess: endianess,
    }
  }

  pub fn buffer(&self) -> &Vec<u8> {
    &self.buffer
  }

  fn calculate_padding_need_and_write_padding(&mut self, typeOctetAlignment: u8) {
    let modulo: u32 = self.buffer.len() as u32 % typeOctetAlignment as u32;
    if modulo != 0 {
      let paddingNeed: u32 = typeOctetAlignment as u32 - modulo;
      //println!("need padding! {}", paddingNeed);
      self.write_pad(paddingNeed);
    } else {
      return;
    }
  }

  fn write_pad(&mut self, byteCount: u32) {
    //println!("PAD byte count: {}", byteCount);
    for _x in 0..byteCount {
      self.buffer.push(0u8);
    }
  }
}

pub fn to_little_endian_binary<T>(value: &T) -> Result<Vec<u8>>
where
  T: Serialize,
{
  let mut CDR_serializer = CDR_serializer {
    buffer: Vec::new(),
    serializationEndianess: Endianess::LittleEndian,
  };
  value.serialize(&mut CDR_serializer)?;
  Ok(CDR_serializer.buffer)
}

pub fn to_big_endian_binary<T>(value: &T) -> Result<Vec<u8>>
where
  T: Serialize,
{
  let mut CDR_serializer = CDR_serializer {
    buffer: Vec::new(),
    serializationEndianess: Endianess::BigEndian,
  };
  value.serialize(&mut CDR_serializer)?;
  Ok(CDR_serializer.buffer)
}

impl<'a> ser::Serializer for &'a mut CDR_serializer {
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
    if v == true {
      self.buffer.push(1u8);
    } else {
      self.buffer.push(0u8);
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
    self.calculate_padding_need_and_write_padding(2);
    if self.serializationEndianess == Endianess::LittleEndian {
      let mut wtr = vec![];
      wtr.write_u16::<LittleEndian>(v).unwrap();
      self.buffer.push(wtr[0]);
      self.buffer.push(wtr[1]);
    } else {
      let mut wtr = vec![];
      wtr.write_u16::<BigEndian>(v).unwrap();
      self.buffer.push(wtr[0]);
      self.buffer.push(wtr[1]);
    }
    Ok(())
  }

  fn serialize_u32(self, v: u32) -> Result<()> {
    self.calculate_padding_need_and_write_padding(4);
    if self.serializationEndianess == Endianess::LittleEndian {
      let mut wtr = vec![];
      wtr.write_u32::<LittleEndian>(v).unwrap();
      self.buffer.push(wtr[0]);
      self.buffer.push(wtr[1]);
      self.buffer.push(wtr[2]);
      self.buffer.push(wtr[3]);
      Ok(())
    } else {
      let mut wtr = vec![];
      wtr.write_u32::<BigEndian>(v).unwrap();
      self.buffer.push(wtr[0]);
      self.buffer.push(wtr[1]);
      self.buffer.push(wtr[2]);
      self.buffer.push(wtr[3]);
      Ok(())
    }
  }

  fn serialize_u64(self, v: u64) -> Result<()> {
    self.calculate_padding_need_and_write_padding(8);
    if self.serializationEndianess == Endianess::LittleEndian {
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
    } else {
      let mut wtr = vec![];
      wtr.write_u64::<BigEndian>(v).unwrap();
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
  }

  fn serialize_i8(self, v: i8) -> Result<()> {
    let mut wtr = vec![];
    wtr.write_i8(v).unwrap();
    self.buffer.push(wtr[0]);
    Ok(())
  }

  fn serialize_i16(self, v: i16) -> Result<()> {
    println!("serialize_i16");
    self.calculate_padding_need_and_write_padding(2);
    if self.serializationEndianess == Endianess::LittleEndian {
      let mut wtr = vec![];
      wtr.write_i16::<LittleEndian>(v).unwrap();
      self.buffer.push(wtr[0]);
      self.buffer.push(wtr[1]);
      Ok(())
    } else {
      let mut wtr = vec![];
      wtr.write_i16::<BigEndian>(v).unwrap();
      self.buffer.push(wtr[0]);
      self.buffer.push(wtr[1]);
      Ok(())
    }
  }

  fn serialize_i32(self, v: i32) -> Result<()> {
    self.calculate_padding_need_and_write_padding(4);
    println!("serialize_i32");
    if self.serializationEndianess == Endianess::LittleEndian {
      let mut wtr = vec![];
      wtr.write_i32::<LittleEndian>(v).unwrap();
      self.buffer.push(wtr[0]);
      self.buffer.push(wtr[1]);
      self.buffer.push(wtr[2]);
      self.buffer.push(wtr[3]);
      Ok(())
    } else {
      let mut wtr = vec![];
      wtr.write_i32::<BigEndian>(v).unwrap();
      self.buffer.push(wtr[0]);
      self.buffer.push(wtr[1]);
      self.buffer.push(wtr[2]);
      self.buffer.push(wtr[3]);
      Ok(())
    }
  }

  fn serialize_i64(self, v: i64) -> Result<()> {
    self.calculate_padding_need_and_write_padding(8);
    if self.serializationEndianess == Endianess::LittleEndian {
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
    } else {
      let mut wtr = vec![];
      wtr.write_i64::<BigEndian>(v).unwrap();
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
  }

  fn serialize_f32(self, _v: f32) -> Result<()> {
    self.calculate_padding_need_and_write_padding(4);
    if self.serializationEndianess == Endianess::LittleEndian {
      let v_bytes = _v.to_bits().to_le_bytes();
      self.buffer.push(v_bytes[0]);
      self.buffer.push(v_bytes[1]);
      self.buffer.push(v_bytes[2]);
      self.buffer.push(v_bytes[3]);
      Ok(())
    } else {
      let v_bytes = _v.to_bits().to_be_bytes();
      self.buffer.push(v_bytes[0]);
      self.buffer.push(v_bytes[1]);
      self.buffer.push(v_bytes[2]);
      self.buffer.push(v_bytes[3]);
      Ok(())
    }
  }
  fn serialize_f64(self, _v: f64) -> Result<()> {
    self.calculate_padding_need_and_write_padding(8);
    if self.serializationEndianess == Endianess::LittleEndian {
      let v_bytes = _v.to_bits().to_le_bytes();
      self.buffer.push(v_bytes[0]);
      self.buffer.push(v_bytes[1]);
      self.buffer.push(v_bytes[2]);
      self.buffer.push(v_bytes[3]);
      self.buffer.push(v_bytes[4]);
      self.buffer.push(v_bytes[5]);
      self.buffer.push(v_bytes[6]);
      self.buffer.push(v_bytes[7]);
      Ok(())
    } else {
      let v_bytes = _v.to_bits().to_be_bytes();
      self.buffer.push(v_bytes[0]);
      self.buffer.push(v_bytes[1]);
      self.buffer.push(v_bytes[2]);
      self.buffer.push(v_bytes[3]);
      self.buffer.push(v_bytes[4]);
      self.buffer.push(v_bytes[5]);
      self.buffer.push(v_bytes[6]);
      self.buffer.push(v_bytes[7]);
      Ok(())
    }
  }

  //An IDL character is represented as a single octet; the code set used for transmission of
  //character data (e.g., TCS-C) between a particular client and server ORBs is determined
  //via the process described in Section 13.10, “Code Set Conversion,”
  fn serialize_char(self, _v: char) -> Result<()> {
    // TODO how to convert RUST 32 bit char to 8 bit safely???
    let charAsinteger = _v as u32;
    let bytes = charAsinteger.to_le_bytes();
    self.buffer.push(bytes[0]);
    Ok(())
  }

  //A string is encoded as an unsigned long indicating the length of the string in octets,
  //followed by the string value in single- or multi-byte form represented as a sequence of
  //octets. The string contents include a single terminating null character. The string
  //length includes the null character, so an empty string has a length of 1.
  fn serialize_str(self, _v: &str) -> Result<()> {
    println!("serialize_str");
    self.calculate_padding_need_and_write_padding(4);
    let count: u32 = _v.chars().count() as u32;
    self.serialize_u32(count + 1).unwrap();
    for c in _v.chars() {
      let charAsinteger = c as u32;
      let bytes = charAsinteger.to_le_bytes();
      self.buffer.push(bytes[0]);
    }
    self.buffer.push(0u8);
    Ok(())
  }

  fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
    for by in _v {
      self.buffer.push(*by);
    }
    Ok(())
  }

  // TODO FUNCTIONS AFTER THIS ARE NOT IMPLEMENTED
  fn serialize_none(self) -> Result<()> {
    Ok(())
  }
  fn serialize_some<T>(self, _: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    println!("serialize_some");
    Ok(())
  }

  fn serialize_unit(self) -> Result<()> {
    println!("serialize_unit");
    Ok(())
  }
  fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
    println!("serialize_unit_struct");
    self.serialize_unit()
  }
  fn serialize_unit_variant(
    self,
    _name: &'static str,
    _variant_index: u32,
    _variant: &'static str,
  ) -> Result<()> {
    println!("serialize_unit_variant");
    println!("unit variant index {:?}", _variant_index);
    self.serialize_u32(_variant_index)
  }
  fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    println!("serialize_newtype_struct");
    value.serialize(self)?;
    Ok(())
  }

  fn serialize_newtype_variant<T>(
    self,
    _name: &'static str,
    _variant_index: u32,
    _variant: &'static str,
    value: &T,
  ) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    println!("serialize_newtype_variant");
    value.serialize(self)
  }

  //Sequences are encoded as an unsigned long value, followed by the elements of the
  //sequence. The initial unsigned long contains the number of elements in the sequence.
  //The elements of the sequence are encoded as specified for their type.
  fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
    self.calculate_padding_need_and_write_padding(4);
    println!("serialize_seq");
    let elementCount = _len.unwrap() as u32;
    if elementCount % 4 != 0 {
      //println!("sequence element count: {}", elementCount);
    }

    self.serialize_u32(elementCount).unwrap();
    Ok(self)
  }
  // if CDR contains fixed length array then number of elements is not written.
  fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
    //println!("serialize tuple");
    println!("serialize_tuple");
    Ok(self)
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
    _variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeTupleVariant> {
    println!("serialize_tuple_variant");
    Ok(self)
  }
  fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
    //println!("serialize map");
    println!("serialize_map");
    Ok(self)
  }
  fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
    self.calculate_padding_need_and_write_padding(4);
    //println!("serialize struct");
    println!("serialize_struct");
    self.serialize_map(Some(len))
  }
  fn serialize_struct_variant(
    self,
    _name: &'static str,
    _variant_index: u32,
    _variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeStructVariant> {
    println!("serialize_struct_variant");
    Ok(self)
  }
}

impl<'a> ser::SerializeSeq for &'a mut CDR_serializer {
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

impl<'a> ser::SerializeTuple for &'a mut CDR_serializer {
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

impl<'a> ser::SerializeTupleStruct for &'a mut CDR_serializer {
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

impl<'a> ser::SerializeTupleVariant for &'a mut CDR_serializer {
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

impl<'a> ser::SerializeMap for &'a mut CDR_serializer {
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

impl<'a> ser::SerializeStruct for &'a mut CDR_serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)?;
    Ok(())
  }

  fn end(self) -> Result<()> {
    Ok(())
  }
}

impl<'a> ser::SerializeStructVariant for &'a mut CDR_serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)?;
    Ok(())
  }

  fn end(self) -> Result<()> {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::serialization::cdrSerializer::to_little_endian_binary;
  use crate::serialization::cdrSerializer::to_big_endian_binary;
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
  use serde::{Serialize, Deserialize};

  #[test]

  fn CDR_serialize_and_deserializesequence_of_structs() {
    // this length is not dividable by 4 so paddings are necessary???
    #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
    pub struct OmaTyyppi {
      firstValue: i16,
      second: u8,
    }
    impl OmaTyyppi {
      pub fn new(firstValue: i16, second: u8) -> OmaTyyppi {
        OmaTyyppi { firstValue, second }
      }
    }

    let sequence_of_structs: Vec<OmaTyyppi> = vec![
      OmaTyyppi::new(1, 23),
      OmaTyyppi::new(2, 34),
      OmaTyyppi::new(-3, 45),
    ];
    let serialized = to_little_endian_binary(&sequence_of_structs).unwrap();
    let deSerialized: Vec<OmaTyyppi> = deserialize_from_little_endian(&serialized).unwrap();
    println!("deSerialized    {:?}", deSerialized);
    println!("serialized    {:?}", serialized);
    assert_eq!(deSerialized, sequence_of_structs);
  }




  #[test]
  fn CDR_serialize_enum(){
    #[derive(Debug, Eq, PartialEq, Serialize,Deserialize)]
    pub enum OmaEnumeration {
      first,
      second,
      third,
      fourth(u8,u8,u8,u8),
      fifth(u8,u8,u16),
      sixth(i16,i16),
      seventh(i32),
      eigth(u32),
      this_should_not_be_valid(u64,u8,u8,u16,String),
      similar_to_fourth(u8,u8,u8,u8),
    }

    let enum_object_1 = OmaEnumeration::first;
    let enum_object_2 = OmaEnumeration::second;
    let enum_object_3 = OmaEnumeration::third;
    let enum_object_4 = OmaEnumeration::fourth(1,2,3,4);
    let enum_object_5 = OmaEnumeration::fifth(5,6,7);
    let enum_object_6 = OmaEnumeration::sixth(-8,9);
    let enum_object_7 = OmaEnumeration::seventh(-100);
    let enum_object_8 = OmaEnumeration::eigth(1000);
    let enum_object_9 = OmaEnumeration::this_should_not_be_valid(1000,1,2,3,String::from("Hejssan allihoppa!"));
    let enum_object_10 = OmaEnumeration::similar_to_fourth(5,6,7,8);

    let serialized_1 = to_little_endian_binary(&enum_object_1).unwrap();
    println!("{:?}", serialized_1);

    let deserialized_1 : OmaEnumeration = deserialize_from_little_endian(serialized_1).unwrap();
    println!("{:?}", deserialized_1);

    let serialized_2 = to_little_endian_binary(&enum_object_2).unwrap();
    println!("{:?}", serialized_2);

    let serialized_3 = to_little_endian_binary(&enum_object_3).unwrap();
    println!("{:?}", serialized_3);

    let serialized_4 = to_little_endian_binary(&enum_object_4).unwrap();
    println!("{:?}", serialized_4);

    let serialized_5 = to_little_endian_binary(&enum_object_5).unwrap();
    println!("{:?}", serialized_5);

    let serialized_6 = to_little_endian_binary(&enum_object_6).unwrap();
    println!("{:?}", serialized_6);

    let serialized_7 = to_little_endian_binary(&enum_object_7).unwrap();
    println!("{:?}", serialized_7);

    let serialized_8 = to_little_endian_binary(&enum_object_8).unwrap();
    println!("{:?}", serialized_8);

    let serialized_9 = to_little_endian_binary(&enum_object_9).unwrap();
    println!("{:?}", serialized_9);

    let serialized_10 = to_little_endian_binary(&enum_object_10).unwrap();
    println!("{:?}", serialized_10);

    

  }



  #[test]
  fn CDR_serialization_example() {
    // look this example https://www.omg.org/spec/DDSI-RTPS/2.2/PDF
    // 10.2.2 Example

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct example {
      a: u32,
      b: [char; 4],
    }

    let o = example {
      a: 1,
      b: ['a', 'b', 'c', 'd'],
    };

    let expected_serialization_le: Vec<u8> = vec![0x01, 0x00, 0x00, 0x00, 0x61, 0x62, 0x63, 0x64];

    let expected_serialization_be: Vec<u8> = vec![0x00, 0x00, 0x00, 0x01, 0x61, 0x62, 0x63, 0x64];

    let serialized_le = to_little_endian_binary(&o).unwrap();
    let serialized_be = to_big_endian_binary(&o).unwrap();
    assert_eq!(serialized_le, expected_serialization_le);
    assert_eq!(serialized_be, expected_serialization_be);
  }

  #[test]
  fn CDR_serializationTest() {
    #[derive(Serialize)]
    struct OmaTyyppi {
      firstValue: u8,
      secondvalue: i8,
      thirdValue: i32,
      fourthValue: u64,
      fifth: bool,
    }

    let mikkiHiiri = OmaTyyppi {
      firstValue: 1,
      secondvalue: -1,
      thirdValue: 23,
      fourthValue: 3434343,
      fifth: true,
    };

    let sarjallistettu = to_little_endian_binary(&mikkiHiiri).unwrap();
    let expected: Vec<u8> = vec![
      0x01, 0xff, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0x67, 0x67, 0x34, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x01,
    ];
    assert_eq!(expected, sarjallistettu)
  }

  fn CDR_serialization_char() {
    #[derive(Serialize)]
    struct OmaTyyppi {
      firstValue: char,
      second: char,
      third: char,
    }
    let mikkiHiiri = OmaTyyppi {
      firstValue: 'a',
      second: 'b',
      third: 'ä',
    };

    let sarjallistettu = to_little_endian_binary(&mikkiHiiri).unwrap();
    let expected: Vec<u8> = vec![0x61, 0x62, 0xe4];
    assert_eq!(expected, sarjallistettu)
  }
  #[test]
  fn CDR_serialization_string() {
    #[derive(Serialize)]
    struct OmaTyyppi<'a> {
      firstValue: &'a str,
    }
    let mikkiHiiri = OmaTyyppi { firstValue: "BLUE" };
    let sarjallistettu = to_little_endian_binary(&mikkiHiiri).unwrap();
    let expected: Vec<u8> = vec![0x05, 0x00, 0x00, 0x00, 0x42, 0x4c, 0x55, 0x45, 0x00];
    assert_eq!(expected, sarjallistettu)
  }

  fn CDR_serialization_little() {
    let number: u16 = 60000;
    let le = to_little_endian_binary(&number).unwrap();
    let be = to_big_endian_binary(&number).unwrap();

    assert_ne!(le, be);
  }

  #[test]
  fn CDR_serialize_seq() {
    #[derive(Serialize)]
    struct OmaTyyppi {
      firstValue: Vec<i32>,
    }
    let mikkiHiiri = OmaTyyppi {
      firstValue: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 123123],
    };
    let sarjallistettu = to_little_endian_binary(&mikkiHiiri).unwrap();
    let expected: Vec<u8> = vec![
      0x0b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x07, 0x00,
      0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0xf3,
      0xe0, 0x01, 0x00,
    ];
    assert_eq!(expected, sarjallistettu)
  }
}
