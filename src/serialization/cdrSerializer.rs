use serde::{ser, Serialize};
use std::marker::PhantomData;
use std::io;
use std::io::Write;

//use serde::{de};
//use std::fmt::{self, Display};
//use error::{Error, Result};
extern crate byteorder;
use crate::serialization::cdrSerializer::byteorder::WriteBytesExt;
use byteorder::{BigEndian,LittleEndian,ByteOrder};

use crate::serialization::error::Error;
use crate::serialization::error::Result;

// This is a wrapper object for a Write object. The wrapper keeps count of bytes written.
// Such a wrapper seemed easier implementation strategy than capturing the return values of all
// write and write_* calls in serializer implementation.
struct CountingWrite<W : io::Write> 
{
  writer: W,
  bytes_written: u64,
}

impl<W> CountingWrite<W>
where
  W: io::Write,
{
  pub fn new(w: W) -> CountingWrite<W> {
    CountingWrite {
      writer: w,
      bytes_written: 0,
    }
  }
  pub fn count(&self) -> u64 {
    self.bytes_written
  }
}

impl<W> io::Write for CountingWrite<W>
where
  W: io::Write,
{
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    match self.writer.write(buf) {
      Ok(c) => {
        self.bytes_written += c as u64;
        Ok(c)
      }
      e => e,
    }
  }
  fn flush(&mut self) -> io::Result<()> {
    self.writer.flush()
  }
}


// ---------------------------------------------------------------------------------
// ---------------------------------------------------------------------------------

/// Parameter W is an io::Write that would receive the serialization
/// Parameter BO is byte order: LittleEndian or BigEndian
pub struct CDR_serializer<W,BO> 
  where W: io::Write
{
  writer: CountingWrite<W>, // serialization destination
  phantom: PhantomData<BO>, // This field exists only to provide use for BO. See PhantomData docs.
}



impl<W,BO> CDR_serializer<W,BO> 
  where BO: ByteOrder,
        W: io::Write,
{
  pub fn new(w: W) -> CDR_serializer<W, BO> {
    CDR_serializer::<W, BO> {
      writer: CountingWrite::<W>::new(w),
      phantom: PhantomData,
    }
  }

  fn calculate_padding_need_and_write_padding(&mut self, typeOctetAlignment: u8) -> Result<()> {
    let modulo: u32 = self.writer.count() as u32 % typeOctetAlignment as u32;
    if modulo != 0 {
      let paddingNeed: u32 = typeOctetAlignment as u32 - modulo;
      //println!("need padding! {}", paddingNeed);
      for _x in 0..paddingNeed {
        self.writer.write_u8(0)? 
      }
    }
    Ok(())
  }
}

pub fn to_writer<T, BO, W>(writer: W, value: &T) -> Result<()>
where
  T: Serialize,
  BO: ByteOrder,
  W: io::Write,
{
  value.serialize(&mut CDR_serializer::<W, BO>::new(writer))
}

pub fn to_bytes<T, BO>(value: &T) -> Result<Vec<u8>>
where
  T: Serialize,
  BO: ByteOrder,
{
  let mut buffer: Vec<u8> = Vec::with_capacity(32); // just some value out of hat.
  to_writer::<T, BO, &mut Vec<u8>>(&mut buffer, &value)?;
  Ok(buffer)
}



// This is private, for unit test cases only
// Public interface should use to_bytes() instead, as it is recommended by serde documentation
fn to_little_endian_binary<T>(value: &T) -> Result<Vec<u8>>
where
  T: Serialize,
{
  to_bytes::<T, LittleEndian>(value)
}

// This is private, for unit test cases only
// Public interface should use to_bytes() instead, as it is recommended by serde documentation
fn to_big_endian_binary<T>(value: &T) -> Result<Vec<u8>>
where
  T: Serialize,
{
  to_bytes::<T, BigEndian>(value)
}



impl<'a,W,BO> ser::Serializer for &'a mut CDR_serializer<W,BO> 
  where BO: ByteOrder,
        W: io::Write,
{
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
      self.writer.write_u8(1u8)?;
    } else {
      self.writer.write_u8(0u8)?;
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
    self.writer.write_u8(v)?;
    Ok(())
  }

  fn serialize_u16(self, v: u16) -> Result<()> {
    self.calculate_padding_need_and_write_padding(2)?;
    self.writer.write_u16::<BO>(v)?;
    Ok(())
  }

  fn serialize_u32(self, v: u32) -> Result<()> {
    self.calculate_padding_need_and_write_padding(4)?;
    self.writer.write_u32::<BO>(v)?;
    Ok(())
  }

  fn serialize_u64(self, v: u64) -> Result<()> {
    self.calculate_padding_need_and_write_padding(8)?;
    self.writer.write_u64::<BO>(v)?;
    Ok(())
  }

  fn serialize_i8(self, v: i8) -> Result<()> {
    self.writer.write_i8(v)?;
    Ok(())
  }

  fn serialize_i16(self, v: i16) -> Result<()> {
    println!("serialize_i16");
    self.calculate_padding_need_and_write_padding(2)?;
    self.writer.write_i16::<BO>(v)?;
    Ok(())
  }

  fn serialize_i32(self, v: i32) -> Result<()> {
    self.calculate_padding_need_and_write_padding(4)?;
    self.writer.write_i32::<BO>(v)?;
    Ok(())
  }

  fn serialize_i64(self, v: i64) -> Result<()> {
    self.calculate_padding_need_and_write_padding(8)?;
    self.writer.write_i64::<BO>(v)?;
    Ok(())
  }

  fn serialize_f32(self, v: f32) -> Result<()> {
    self.calculate_padding_need_and_write_padding(4)?;
    self.writer.write_f32::<BO>(v)?;
    Ok(())
  }
  fn serialize_f64(self, v: f64) -> Result<()> {
    self.calculate_padding_need_and_write_padding(8)?;
    self.writer.write_f64::<BO>(v)?;
    Ok(())
  }

  //An IDL character is represented as a single octet; the code set used for transmission of
  //character data (e.g., TCS-C) between a particular client and server ORBs is determined
  //via the process described in Section 13.10, “Code Set Conversion,”
  fn serialize_char(self, v: char) -> Result<()> {
    // Rust "char" means a 32-bit Unicode code point.
    // IDL & CDR "char" means an octet.
    // We are here actually serializing the 32-bit quantity.
    // If we want to serialize CDR "char", then the corresponding Rust type is "u8".
    self.serialize_u32(v as u32)?;
    Ok(())
  }

  //A string is encoded as an unsigned long indicating the length of the string in octets,
  //followed by the string value in single- or multi-byte form represented as a sequence of
  //octets. The string contents include a single terminating null character. The string
  //length includes the null character, so an empty string has a length of 1.
  fn serialize_str(self, v: &str) -> Result<()> {
    self.calculate_padding_need_and_write_padding(4)?;
    let byte_count: u32 = v.as_bytes().len() as u32 + 1;
    self.serialize_u32(byte_count)?; // +1 for terminator
    self.writer.write(v.as_bytes())?;
    self.writer.write_u8(0)?; // CDR spec requires a null terminator
    Ok(())
    // The end result is not UTF-8-encoded string, but how could we do better in CDR?
  }

  fn serialize_bytes(self, v: &[u8]) -> Result<()> {
    self.writer.write(v)?;
    Ok(())
  }

  fn serialize_none(self) -> Result<()> {
    self.serialize_u32(0) // None is the first variant
  }

  fn serialize_some<T>(self, t: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.serialize_u32(1)?; // Some is the second variant
    t.serialize(self)?;
    Ok(())
  }

  // Unit contains no data, but the CDR spec has no concept of types that carry no data.
  // We decide to serialize this as nothing.
  fn serialize_unit(self) -> Result<()> {
    // nothing
    Ok(())
  }

  fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
    self.serialize_unit()
  }


  // CDR 15.3.2.6:
  // Enum values are encoded as unsigned longs. The numeric values associated with enum
  // identifiers are determined by the order in which the identifiers appear in the enum
  // declaration. The first enum identifier has the numeric value zero (0). Successive enum
  // identifiers take ascending numeric values, in order of declaration from left to right.
  fn serialize_unit_variant(
    self,
    _name: &'static str,
    variant_index: u32,
    _variant: &'static str,
  ) -> Result<()> {
    self.serialize_u32(variant_index)
  }

  // In CDR, this would be a special case of struct.
  fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

  // As this is technically an enum, we treat this as union with one variat. 
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

    self.serialize_u32(0)?;
    value.serialize(self)
  }

  //Sequences are encoded as an unsigned long value, followed by the elements of the
  //sequence. The initial unsigned long contains the number of elements in the sequence.
  //The elements of the sequence are encoded as specified for their type.
  //
  // Serde calls this to start sequence. Then it calls serialize_element() for each element, and
  // finally calls end().
  fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
    match len {
      None => Err(Error::SequenceLengthUnknown),
      Some( elem_count ) => {
        self.serialize_u32(elem_count as u32 )?;
        Ok(self)
      }
    } // match
  } // fn

  // if CDR contains fixed length array then number of elements is not written.
  fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
    // nothing to be done here
    Ok(self)
  }

  fn serialize_tuple_struct(
    self,
    _name: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeTupleStruct> {
    // (tuple) struct length is not written. Nothing to be done.
    Ok(self)
  }

  // This is technically enum/union, so write the variant index.
  fn serialize_tuple_variant(
    self,
    _name: &'static str,
    variant_index: u32,
    _variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeTupleVariant> {
    self.serialize_u32(variant_index)?;
    Ok(self)
  }

  // CDR spec from year 2002 does not yet know about maps.
  // We make an educated guess that the design is similar to sequences:
  // First the number of key-value pairs, then each entry as a (key,value)-pair.
  fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
    //println!("serialize_map");
    match len {
      None => Err(Error::SequenceLengthUnknown),
      Some( elem_count ) => {
        self.serialize_u32(elem_count as u32 )?;
        Ok(self)
      }
    } // match
  }

  // Similar to tuple. No need to mark the beginning.
  fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
    self.calculate_padding_need_and_write_padding(4)?;
    //println!("serialize struct");
    // nothing to be done.
    Ok(self)
  }

  // Same as tuple variant: Serialize variant index. Serde will then serialize fields.
  fn serialize_struct_variant(
    self,
    _name: &'static str,
    variant_index: u32,
    _variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeStructVariant> {
    self.serialize_u32(variant_index)?;
    Ok(self)
  }
}

impl<'a, W: io::Write, BO: ByteOrder> ser::SerializeSeq for &'a mut CDR_serializer<W, BO> {
  type Ok = ();
  type Error = Error;

  fn serialize_element<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> { Ok(()) }
}

impl<'a, W: io::Write, BO: ByteOrder> ser::SerializeTuple for &'a mut CDR_serializer<W, BO> {
  type Ok = ();
  type Error = Error;

  fn serialize_element<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> { Ok(()) }
}

impl<'a, W: io::Write, BO: ByteOrder> ser::SerializeTupleStruct for &'a mut CDR_serializer<W, BO> {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> { Ok(()) }
}

impl<'a, W: io::Write, BO: ByteOrder> ser::SerializeTupleVariant for &'a mut CDR_serializer<W, BO> {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }
  fn end(self) -> Result<()> { Ok(()) }
}

impl<'a, W: io::Write, BO: ByteOrder> ser::SerializeMap for &'a mut CDR_serializer<W, BO> {
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

  fn end(self) -> Result<()> { Ok(()) }
}

impl<'a, W: io::Write, BO: ByteOrder> ser::SerializeStruct for &'a mut CDR_serializer<W, BO> {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)?;
    Ok(())
  }

  fn end(self) -> Result<()> { Ok(()) }
}

impl<'a, W: io::Write, BO: ByteOrder> ser::SerializeStructVariant
  for &'a mut CDR_serializer<W, BO>
{
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)?;
    Ok(())
  }

  fn end(self) -> Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests {
  use crate::serialization::cdrSerializer::to_little_endian_binary;
  use crate::serialization::cdrSerializer::to_big_endian_binary;
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
  use serde::{Serialize, Deserialize};
  use serde_repr::{Serialize_repr, Deserialize_repr};

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
  fn CDR_serialize_enum() {
    //Enum values are encoded as unsigned longs. The numeric values associated with enum
    //identifiers are determined by the order in which the identifiers appear in the enum
    //declaration. The first enum identifier has the numeric value zero (0). Successive enum
    //identifiers are take ascending numeric values, in order of declaration from left to right.
    // -> Only C type "classic enumerations" are possible to be serialized.
    // use Serialize_repr and Deserialize_repr when enum member has value taht is not same as unit variant index
    // (when specified explicitly in code with assing)
    #[derive(Debug, Eq, PartialEq, Serialize_repr, Deserialize_repr)]
    #[repr(u32)]
    pub enum OmaEnumeration {
      first,
      second,
      third,
      //fourth(u8,u8,u8,u8),
      //fifth(u8,u8,u16),
      //sixth(i16,i16),
      seventhHundred = 700,
      //eigth(u32),
      //this_should_not_be_valid(u64,u8,u8,u16,String),
      //similar_to_fourth(u8,u8,u8,u8),
    }

    let enum_object_1 = OmaEnumeration::first;
    let enum_object_2 = OmaEnumeration::second;
    let enum_object_3 = OmaEnumeration::third;
    //let enum_object_4 = OmaEnumeration::fourth(1,2,3,4);
    //let enum_object_5 = OmaEnumeration::fifth(5,6,7);
    //let enum_object_6 = OmaEnumeration::sixth(-8,9);
    let enum_object_7 = OmaEnumeration::seventhHundred;
    //let enum_object_8 = OmaEnumeration::eigth(1000);
    //let enum_object_9 = OmaEnumeration::this_should_not_be_valid(1000,1,2,3,String::from("Hejssan allihoppa!"));
    //let enum_object_10 = OmaEnumeration::similar_to_fourth(5,6,7,8);

    let serialized_1 = to_little_endian_binary(&enum_object_1).unwrap();
    println!("{:?}", serialized_1);
    let u32Value_1: u32 = deserialize_from_little_endian(&serialized_1).unwrap();
    let deserialized_1: OmaEnumeration = deserialize_from_little_endian(&serialized_1).unwrap();
    println!("Deserialized 1: {:?}", deserialized_1);
    assert_eq!(deserialized_1, enum_object_1);
    assert_eq!(u32Value_1, 0);

    let serialized_2 = to_little_endian_binary(&enum_object_2).unwrap();
    println!("{:?}", serialized_2);
    let u32Value_2: u32 = deserialize_from_little_endian(&serialized_2).unwrap();
    let deserialized_2: OmaEnumeration = deserialize_from_little_endian(&serialized_2).unwrap();
    println!("Deserialized 2: {:?}", deserialized_2);
    assert_eq!(deserialized_2, enum_object_2);
    assert_eq!(u32Value_2, 1);

    let serialized_3 = to_little_endian_binary(&enum_object_3).unwrap();
    println!("{:?}", serialized_3);
    let deserialized_3: OmaEnumeration = deserialize_from_little_endian(&serialized_3).unwrap();
    let u32Value_3: u32 = deserialize_from_little_endian(&serialized_3).unwrap();
    println!("Deserialized 3: {:?}", deserialized_3);
    assert_eq!(deserialized_3, enum_object_3);
    assert_eq!(u32Value_3, 2);

    /*
    let serialized_4 = to_little_endian_binary(&enum_object_4).unwrap();
    println!("{:?}", serialized_4);
    let deserialized_4 : OmaEnumeration = deserialize_from_little_endian(serialized_4).unwrap();
    println!("Deserialized 4: {:?}", deserialized_4);
    */

    let serialized_7 = to_little_endian_binary(&enum_object_7).unwrap();
    println!("{:?}", serialized_7);
    let deserialized_7: OmaEnumeration = deserialize_from_little_endian(&serialized_7).unwrap();
    let u32Value_7: u32 = deserialize_from_little_endian(&serialized_7).unwrap();
    println!("Deserialized 7: {:?}", deserialized_7);
    assert_eq!(deserialized_7, enum_object_7);
    assert_eq!(u32Value_7, 700);

    /*
    let serialized_5 = to_little_endian_binary(&enum_object_5).unwrap();
    println!("{:?}", serialized_5);

    let serialized_6 = to_little_endian_binary(&enum_object_6).unwrap();
    println!("{:?}", serialized_6);



    let serialized_8 = to_little_endian_binary(&enum_object_8).unwrap();
    println!("{:?}", serialized_8);

    let serialized_9 = to_little_endian_binary(&enum_object_9).unwrap();
    println!("{:?}", serialized_9);

    let serialized_10 = to_little_endian_binary(&enum_object_10).unwrap();
    println!("{:?}", serialized_10);

    */
  }

  #[test]
  fn CDR_serialization_example() {
    // look this example https://www.omg.org/spec/DDSI-RTPS/2.2/PDF
    // 10.2.2 Example

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct example {
      a: u32,
      b: [u8; 4],
    }

    let o = example {
      a: 1,
      b: [b'a', b'b', b'c', b'd'],
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
