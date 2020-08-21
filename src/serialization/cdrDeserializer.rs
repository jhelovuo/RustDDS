use byteorder::{ByteOrder, LittleEndian, BigEndian};
use serde::Deserialize;
use serde::{
  de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
  },
};
use crate::serialization::error::Error;
use crate::serialization::error::Result;

#[derive(PartialEq)]
enum endianess {
  littleEndian,
  bigEndian,
}

pub struct CDR_deserializer {
  input: Vec<u8>,
  DeserializationEndianess: endianess,
  serializedDataCount: u32,
}

impl<'de> CDR_deserializer {
  pub fn deserialize_from_little_endian(input: Vec<u8>) -> Self {
    CDR_deserializer {
      input,
      DeserializationEndianess: endianess::littleEndian,
      serializedDataCount: 0,
    }
  }

  pub fn deserialize_from_big_endian(input: Vec<u8>) -> Self {
    CDR_deserializer {
      input,
      DeserializationEndianess: endianess::bigEndian,
      serializedDataCount: 0,
    }
  }

  fn remove_first_byte_from_input(&mut self) {
    self.serializedDataCount = self.serializedDataCount + 1;
    self.input.remove(0);
  }

  fn calculate_padding_count_from_written_bytes_and_remove(&mut self, typeOctetAligment: u8) {
    let modulo = self.serializedDataCount % typeOctetAligment as u32;

    if modulo != 0 {
      let padding: u32 = typeOctetAligment as u32 - modulo;
      println!("need to remove padding! {}", padding);
      self.remove_padding_bytes_from_end(padding);
    } else {
      return;
    }
  }

  fn remove_padding_bytes_from_end(&mut self, padCount: u32) {
    println!("remove padding {}", padCount);
    for _a in 0..padCount {
      self.remove_first_byte_from_input();
    }
  }
}

pub fn deserialize_from_little_endian<'a, T>(s: &Vec<u8>) -> Result<T>
where
  T: Deserialize<'a>,
{
  let mut deserializer = CDR_deserializer::deserialize_from_little_endian(s.to_vec());
  let t = T::deserialize(&mut deserializer)?;
  if deserializer.input.is_empty() {
    Ok(t)
  } else {
    Ok(t)
    //panic!()
  }
}

pub fn deserialize_from_big_endian<'a, T>(s: &Vec<u8>) -> Result<T>
where
  T: Deserialize<'a>,
{
  let mut deserializer = CDR_deserializer::deserialize_from_big_endian(s.to_vec());
  let t = T::deserialize(&mut deserializer)?;
  if deserializer.input.is_empty() {
    Ok(t)
  } else {
    Ok(t)
    //panic!()
  }
}

impl<'de> CDR_deserializer {
  // Look at the first byte in the input without consuming it.
  fn peek_byte(&mut self) -> Result<&u8> {
    self.input.first().ok_or(Error::Eof)
  }

  fn check_if_bytes_left(&mut self) -> bool {
    let someValueFound = self.input.first().ok_or(Error::Eof);
    if someValueFound.is_ok() {
      true
    } else {
      false
    }
  }

  fn get_input_size(&mut self) -> u64 {
    self.input.len() as u64
  }

  // Consume the first byte in the input.
  fn next_byte(&mut self) -> Result<u8> {
    let by = self.input[0];
    self.remove_first_byte_from_input();
    Ok(by)
  }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut CDR_deserializer {
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
    let result: bool;
    if self.next_byte().unwrap() == 1u8 {
      result = true;
    } else {
      result = false;
    }
    visitor.visit_bool(result)
  }

  fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let firstByte = self.next_byte().unwrap();
    let i8Byte = firstByte as i8;
    visitor.visit_i8(i8Byte)
  }

  fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(2);
    if self.DeserializationEndianess == endianess::littleEndian {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let bytes: [u8; 2] = [by0, by1];
      let result: i16 = LittleEndian::read_i16(&bytes);
      visitor.visit_i16(result)
    } else {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let bytes: [u8; 2] = [by0, by1];
      let result: i16 = BigEndian::read_i16(&bytes);
      visitor.visit_i16(result)
    }
  }

  fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(4);
    if self.DeserializationEndianess == endianess::littleEndian {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let bytes: [u8; 4] = [by0, by1, by2, by3];
      let result: i32 = LittleEndian::read_i32(&bytes);
      visitor.visit_i32(result)
    } else {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let bytes: [u8; 4] = [by0, by1, by2, by3];
      let result: i32 = BigEndian::read_i32(&bytes);
      visitor.visit_i32(result)
    }
  }

  fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(8);
    if self.DeserializationEndianess == endianess::littleEndian {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let by4 = self.next_byte().unwrap();
      let by5 = self.next_byte().unwrap();
      let by6 = self.next_byte().unwrap();
      let by7 = self.next_byte().unwrap();
      let bytes: [u8; 8] = [by0, by1, by2, by3, by4, by5, by6, by7];
      let result: i64 = LittleEndian::read_i64(&bytes);
      visitor.visit_i64(result)
    } else {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let by4 = self.next_byte().unwrap();
      let by5 = self.next_byte().unwrap();
      let by6 = self.next_byte().unwrap();
      let by7 = self.next_byte().unwrap();
      let bytes: [u8; 8] = [by0, by1, by2, by3, by4, by5, by6, by7];
      let result: i64 = BigEndian::read_i64(&bytes);
      visitor.visit_i64(result)
    }
  }

  fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by = self.next_byte().unwrap();
    visitor.visit_u8(by)
  }

  fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(2);
    if self.DeserializationEndianess == endianess::littleEndian {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let bytes: [u8; 2] = [by0, by1];
      let result = LittleEndian::read_u16(&bytes);
      visitor.visit_u16(result)
    } else {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let bytes: [u8; 2] = [by0, by1];
      let result = BigEndian::read_u16(&bytes);
      visitor.visit_u16(result)
    }
  }

  fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(4);
    if self.DeserializationEndianess == endianess::littleEndian {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let bytes: [u8; 4] = [by0, by1, by2, by3];
      let result = LittleEndian::read_u32(&bytes);
      visitor.visit_u32(result)
    } else {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let bytes: [u8; 4] = [by0, by1, by2, by3];
      let result = BigEndian::read_u32(&bytes);
      visitor.visit_u32(result)
    }
  }

  fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(8);
    if self.DeserializationEndianess == endianess::littleEndian {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let by4 = self.next_byte().unwrap();
      let by5 = self.next_byte().unwrap();
      let by6 = self.next_byte().unwrap();
      let by7 = self.next_byte().unwrap();
      let bytes: [u8; 8] = [by0, by1, by2, by3, by4, by5, by6, by7];
      let result = LittleEndian::read_u64(&bytes);
      visitor.visit_u64(result)
    } else {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let by4 = self.next_byte().unwrap();
      let by5 = self.next_byte().unwrap();
      let by6 = self.next_byte().unwrap();
      let by7 = self.next_byte().unwrap();
      let bytes: [u8; 8] = [by0, by1, by2, by3, by4, by5, by6, by7];
      let result = BigEndian::read_u64(&bytes);
      visitor.visit_u64(result)
    }
  }

  fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(4);
    if self.DeserializationEndianess == endianess::littleEndian {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let bytes: [u8; 4] = [by0, by1, by2, by3];
      let result = LittleEndian::read_f32(&bytes);
      _visitor.visit_f32(result)
    } else {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let bytes: [u8; 4] = [by0, by1, by2, by3];
      let result = BigEndian::read_f32(&bytes);
      _visitor.visit_f32(result)
    }
  }

  fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(8);
    if self.DeserializationEndianess == endianess::littleEndian {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let by4 = self.next_byte().unwrap();
      let by5 = self.next_byte().unwrap();
      let by6 = self.next_byte().unwrap();
      let by7 = self.next_byte().unwrap();
      let bytes: [u8; 8] = [by0, by1, by2, by3, by4, by5, by6, by7];
      let result: f64 = LittleEndian::read_f64(&bytes);
      _visitor.visit_f64(result)
    } else {
      let by0 = self.next_byte().unwrap();
      let by1 = self.next_byte().unwrap();
      let by2 = self.next_byte().unwrap();
      let by3 = self.next_byte().unwrap();
      let by4 = self.next_byte().unwrap();
      let by5 = self.next_byte().unwrap();
      let by6 = self.next_byte().unwrap();
      let by7 = self.next_byte().unwrap();
      let bytes: [u8; 8] = [by0, by1, by2, by3, by4, by5, by6, by7];
      let result: f64 = BigEndian::read_f64(&bytes);
      _visitor.visit_f64(result)
    }
  }

  fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by0 = self.next_byte().unwrap();
    _visitor.visit_char(by0 as char)
  }

  fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_str");
    self.calculate_padding_count_from_written_bytes_and_remove(4);
    // first is information about how long string is in bytes.
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let by2 = self.next_byte().unwrap();
    let by3 = self.next_byte().unwrap();
    let bytes: [u8; 4] = [by0, by1, by2, by3];
    let stringByteCount: u32;
    if self.DeserializationEndianess == endianess::littleEndian {
      stringByteCount = LittleEndian::read_u32(&bytes);
    } else {
      stringByteCount = BigEndian::read_u32(&bytes);
    }
    let buildString: String;
    let mut chars: Vec<char> = [].to_vec();

    println!(
      "trying to deserialize string of byte length: {}",
      stringByteCount
    );

    // last byte is always 0 and it can be ignored.
    if stringByteCount > 0 {
      for _byte in 0..stringByteCount - 1 {
        let c = self.next_byte().unwrap() as char;
        chars.push(c);
      }
    }

    // here need to call next byte to remove trailing 0 from buffer.
    self.remove_first_byte_from_input();
    buildString = chars.into_iter().collect();

    // TODO check is this correct way to create string literals. This is propably not correct!!!
    fn string_to_static_str(s: String) -> &'static str {
      Box::leak(s.into_boxed_str())
    }
    visitor.visit_borrowed_str(string_to_static_str(buildString))
  }

  fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_string");
    self.deserialize_str(visitor)
  }

  fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_byte_buf(self.input.clone())
  }

  fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_option");
    unimplemented!()
  }

  fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_unit");
    unimplemented!()
  }

  fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_unit_struct");
    self.deserialize_unit(visitor)
  }

  fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_newtype_struct");
    visitor.visit_newtype_struct(self)
  }

  ///Sequences are encoded as an unsigned long value, followed by the elements of the
  //sequence. The initial unsigned long contains the number of elements in the sequence.
  //The elements of the sequence are encoded as specified for their type.
  fn deserialize_seq<V>(mut self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_seq");
    self.calculate_padding_count_from_written_bytes_and_remove(4);
    if self.serializedDataCount % 2 != 0 {
      println!("seq does not start a multiple of 2 !");
    }

    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let by2 = self.next_byte().unwrap();
    let by3 = self.next_byte().unwrap();
    let bytes: [u8; 4] = [by0, by1, by2, by3];
    let elementCount: u32;
    if self.DeserializationEndianess == endianess::littleEndian {
      elementCount = LittleEndian::read_u32(&bytes);
    } else {
      elementCount = BigEndian::read_u32(&bytes);
    }
    println!("seq length: {}", elementCount);
    let res = _visitor.visit_seq(SequenceHelper::new(&mut self, elementCount, true));
    res
  }

  // if sequence is fixed length array then number of elements is not included
  fn deserialize_tuple<V>(mut self, _len: usize, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_tuple");
    visitor.visit_seq(SequenceHelper::new(&mut self, _len as u32, false))
  }

  fn deserialize_tuple_struct<V>(
    self,
    _name: &'static str,
    _len: usize,
    _visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_tuple_struct");
    unimplemented!()
  }

  fn deserialize_map<V>(/*mut*/ self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn deserialize_struct<V>(
    mut self,
    _name: &'static str,
    _fields: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_struct");
    self.calculate_padding_count_from_written_bytes_and_remove(4);
    /*
    println!(
      "deserialize struct! it has num of fields: {} ",
      _fields.len()
    );*/
    for _f in _fields {
      //println!("field: {} ", f);
    }
    visitor.visit_seq(SequenceHelper::new(&mut self, _fields.len() as u32, false))
  }

  ///Enum values are encoded as unsigned longs. (u32)
  /// The numeric values associated with enum identifiers are determined by the order in which the identifiers appear in the enum declaration.
  /// The first enum identifier has the numeric value zero (0). Successive enum identifiers take ascending numeric values, in order of declaration from left to right.
  fn deserialize_enum<V>(
    mut self,
    _name: &'static str,
    _variants: &'static [&'static str],
    _visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(4);
    println!("_name {:?}", _name);
    println!("variants {:?}", _variants);
    return _visitor.visit_enum(EnumerationHelper::new(&mut self));
  }

  /// An identifier in Serde is the type that identifies a field of a struct or
  /// the variant of an enum. In JSON, struct fields and enum variants are
  /// represented as strings. In other formats they may be represented as
  /// numeric indices.
  fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_identifier");
    self.deserialize_u32(visitor)
  }

  fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize_ignored_any");
    self.deserialize_any(visitor)
  }
}

struct EnumerationHelper<'a> {
  de: &'a mut CDR_deserializer,
}
impl<'a, 'de> EnumerationHelper<'a> {
  fn new(de: &'a mut CDR_deserializer) -> Self {
    EnumerationHelper { de }
  }
}

impl<'de, 'a> EnumAccess<'de> for EnumerationHelper<'a> {
  type Error = Error;
  type Variant = Self;

  fn variant_seed<V>(self, _seed: V) -> Result<(V::Value, Self::Variant)>
  where
    V: DeserializeSeed<'de>,
  {
    println!("EnumAccess variant_seed");

    let by0 = self.de.next_byte().unwrap();
    let by1 = self.de.next_byte().unwrap();
    let by2 = self.de.next_byte().unwrap();
    let by3 = self.de.next_byte().unwrap();
    let bytes: [u8; 4] = [by0, by1, by2, by3];
    let enum_number_value: u32;
    if self.de.DeserializationEndianess == endianess::littleEndian {
      enum_number_value = LittleEndian::read_u32(&bytes);
    } else {
      enum_number_value = BigEndian::read_u32(&bytes);
    }
    let val: Result<_> = _seed.deserialize(enum_number_value.into_deserializer());
    return Ok((val?, self));
  }
}

impl<'de, 'a> VariantAccess<'de> for EnumerationHelper<'a> {
  type Error = Error;

  fn unit_variant(self) -> Result<()> {
    println!("VariantAccess unit_variant");
    Ok(())
    //unimplemented!()
  }

  fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value>
  where
    T: DeserializeSeed<'de>,
  {
    println!("VariantAccess newtype_variant_seed");
    unimplemented!();
    //seed.deserialize()
  }

  fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("VariantAccess tuple_variant");
    unimplemented!();
    //self.de.deserialize_seq(/*self.de,*/ visitor)
  }

  fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("VariantAccess struct_variant");
    unimplemented!();
    //self.de.deserialize_map(/*self.de,*/ visitor)
  }
  fn newtype_variant<T>(self) -> Result<T>
  where
    T: Deserialize<'de>,
  {
    println!("VariantAccess newtype_variant");
    unimplemented!();
    //self.newtype_variant_seed(self);
    //self.de.newtype_variant_seed(std::marker::PhantomData)
  }
}

struct SequenceHelper<'a> {
  de: &'a mut CDR_deserializer,
  first: bool,
  elementCounter: u64,
  inputSizeFirstElement: u64,
  inputSizeBeforeLastElement: u64,
  expectedCount: u32,
  isVariableSizeSequence: bool,
}

impl<'a, 'de> SequenceHelper<'a> {
  fn new(de: &'a mut CDR_deserializer, expectedCount: u32, isVariableSizeSequence: bool) -> Self {
    SequenceHelper {
      de,
      first: true,
      elementCounter: 0,
      inputSizeFirstElement: 0,
      inputSizeBeforeLastElement: 0,
      expectedCount: expectedCount,
      isVariableSizeSequence: isVariableSizeSequence,
    }
  }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> SeqAccess<'de> for SequenceHelper<'a> {
  type Error = Error;

  fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
  where
    T: DeserializeSeed<'de>,
  {
    if self.first == true && self.isVariableSizeSequence {
      self.inputSizeFirstElement = self.de.get_input_size() as u64;
      println!(
        "firstElement size: {} expected element count: {}",
        self.inputSizeFirstElement, self.expectedCount
      );
    }
    if self.elementCounter == self.expectedCount as u64 {
      println!("STOP SEQ");
      return Ok(None);
    }

    if self.isVariableSizeSequence {
      println!("seq element number: {}", self.elementCounter);
    }
    self.elementCounter = self.elementCounter + 1;

    //if self.isVariableSizeSequence && self.first == false && self.elementCounter == 2 {
    //  println!("elementSize: {}", (self.inputSizeFirstElement - self.de.get_input_size() as u64) as i64);
    //}

    /*
     if self.elementCounter == self.expectedCount as u64 && self.isVariableSizeSequence{

       self.inputSizeBeforeLastElement = self.de.get_input_size() as u64;
       println!("lastElement now! {}", self.inputSizeBeforeLastElement);
     }
     if self.isVariableSizeSequence {
       println!("next element. Counter now {}",  self.elementCounter);
     }
    */

    // Check if there are no more elements.
    if self.de.check_if_bytes_left() == false {
      return Ok(None);
    }

    self.first = false;
    // Deserialize an array element.
    seed.deserialize(&mut *self.de).map(Some)
  }
}

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, 'a> MapAccess<'de> for SequenceHelper<'a> {
  type Error = Error;

  fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
  where
    K: DeserializeSeed<'de>,
  {
    // Check if there are no more elements.
    if self.de.check_if_bytes_left() == false {
      return Ok(None);
    }
    self.first = false;
    // Deserialize a map key.
    seed.deserialize(&mut *self.de).map(Some)
  }

  fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
  where
    V: DeserializeSeed<'de>,
  {
    // Deserialize a map value.
    seed.deserialize(&mut *self.de)
  }
}

#[cfg(test)]
mod tests {
  use crate::serialization::cdrSerializer::to_bytes;
  use byteorder::{BigEndian,LittleEndian};
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
  use crate::serialization::cdrDeserializer::deserialize_from_big_endian;
  use serde::{Serialize, Deserialize};
  use std::any::type_name;

  #[test]
  fn CDR_Deserialization_struct() {
    //IDL
    /*
    struct OmaTyyppi
    {
     octet first;
    octet second;
    long third;
    unsigned long long fourth;
    boolean fifth;
    float sixth;
    boolean seventh;
    sequence<long> eigth;
    sequence<octet> ninth;
    sequence<short> tenth;
    sequence<long long> eleventh;
    unsigned short twelwe [3];
    string thirteen;
    };
    */

    /*

      ser_var.first(1);
    ser_var.second(-3);
    ser_var.third(-5000);
    ser_var.fourth(1234);
    ser_var.fifth(true);
    ser_var.sixth(-6.6);
    ser_var.seventh(true);
    ser_var.eigth({1,2});
    ser_var.ninth({1});
    ser_var.tenth({5,-4,3,-2,1});
    ser_var.eleventh({});
    ser_var.twelwe({3,2,1});
    ser_var.thirteen("abc");

      */
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct OmaTyyppi<'a> {
      firstValue: u8,
      secondvalue: i8,
      thirdValue: i32,
      fourthValue: u64,
      fifth: bool,
      sixth: f32,
      seventh: bool,
      eigth: Vec<i32>,
      ninth: Vec<u8>,
      tenth: Vec<i16>,
      eleventh: Vec<i64>,
      twelwe: [u16; 3],
      thirteen: &'a str,
    }

    let mikkiHiiri = OmaTyyppi {
      firstValue: 1,
      secondvalue: -3,
      thirdValue: -5000,
      fourthValue: 1234u64,
      fifth: true,
      sixth: -6.6f32,
      seventh: true,
      eigth: vec![1, 2],
      ninth: vec![1],
      tenth: vec![5, -4, 3, -2, 1],
      eleventh: vec![],
      twelwe: [3, 2, 1],
      thirteen: "abc",
    };

    let expected_serialized_result: Vec<u8> = vec![
      0x01, 0xfd, 0x00, 0x00, 0x78, 0xec, 0xff, 0xff, 0xd2, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x33, 0x33, 0xd3, 0xc0, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00,
      0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
      0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x05, 0x00, 0xfc, 0xff, 0x03, 0x00, 0xfe, 0xff,
      0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x61, 0x62, 0x63, 0x00,
    ];

    let sarjallistettu = to_bytes::<OmaTyyppi,LittleEndian>(&mikkiHiiri).unwrap();
    //println!("{:?}",sarjallistettu);

    for x in 0..expected_serialized_result.len() {
      if expected_serialized_result[x] != sarjallistettu[x] {
        println!("index: {}", x);
      }
    }
    assert_eq!(sarjallistettu, expected_serialized_result);
    println!("serialization successfull!");

    let rakennettu: OmaTyyppi =
      deserialize_from_little_endian(&expected_serialized_result).unwrap();
    assert_eq!(rakennettu, mikkiHiiri);
    println!("deserialized: {:?}", rakennettu);
  }

  #[test]

  fn CDR_Deserialization_user_defined_data() {
    // look this example https://www.omg.org/spec/DDSI-RTPS/2.3/PDF
    //10.7 Example for User-defined Topic Data
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct ShapeType<'a> {
      color: &'a str,
      x: i32,
      y: i32,
      size: i32,
    }

    let message = ShapeType {
      color: "BLUE",
      x: 34,
      y: 100,
      size: 24,
    };

    let expected_serialized_result: Vec<u8> = vec![
      0x05, 0x00, 0x00, 0x00, 0x42, 0x4c, 0x55, 0x45, 0x00, 0x00, 0x00, 0x00, 0x22, 0x00, 0x00,
      0x00, 0x64, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
    ];

    let serialized = to_bytes::<ShapeType,LittleEndian>(&message).unwrap();
    assert_eq!(serialized, expected_serialized_result);
    let deserializedMessage: ShapeType = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(deserializedMessage, message)
  }

  #[test]

  fn CDR_Deserialization_serialization_topic_name() {
    // look this example https://www.omg.org/spec/DDSI-RTPS/2.3/PDF
    //10.6 Example for Built-in Endpoint Data
    // this is just CRD topic name strings

    // TODO what about padding??
    let recievedCDRString: Vec<u8> = vec![
      0x07, 0x00, 0x00, 0x00, 0x053, 0x71, 0x75, 0x61, 0x72, 0x65, 0x00, /* 0x00, */
    ];

    let deserializedMessage: &str = deserialize_from_little_endian(&recievedCDRString).unwrap();
    println!("{:?}", deserializedMessage);
    assert_eq!("Square", deserializedMessage);

    let recievedCDRString2: Vec<u8> = vec![
      0x0A, 0x00, 0x00, 0x00, 0x53, 0x68, 0x61, 0x70, 0x65, 0x54, 0x79, 0x70, 0x65,
      0x00, /* 0x00, 0x00, */
    ];

    let deserializedMessage2: &str = deserialize_from_little_endian(&recievedCDRString2).unwrap();
    println!("{:?}", deserializedMessage2);
    assert_eq!("ShapeType", deserializedMessage2);
  }

  #[test]
  fn CDR_Deserialization_example_struct() {
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

    let serialized_le: Vec<u8> = vec![0x01, 0x00, 0x00, 0x00, 0x61, 0x62, 0x63, 0x64];

    let serialized_be: Vec<u8> = vec![0x00, 0x00, 0x00, 0x01, 0x61, 0x62, 0x63, 0x64];

    let deserialized_le: example = deserialize_from_little_endian(&serialized_le).unwrap();
    let deserialized_be: example = deserialize_from_big_endian(&serialized_be).unwrap();
    let serializedO_le = to_bytes::<example,LittleEndian>(&o).unwrap();
    let serializedO_be = to_bytes::<example,BigEndian>(&o).unwrap();

    assert_eq!(
      serializedO_le,
      vec![0x01, 0x00, 0x00, 0x00, 0x61, 0x62, 0x63, 0x64,]
    );

    assert_eq!(
      serializedO_be,
      vec![0x00, 0x00, 0x00, 0x01, 0x61, 0x62, 0x63, 0x64,]
    );

    println!("serialization success");

    assert_eq!(deserialized_le, o);
    assert_eq!(deserialized_be, o);
    println!("deserialition success");
  }

  #[test]

  fn CDR_Deserialization_serialization_payload_shapes() {
    // This test uses wireshark captured shapes demo part of serialized message as recieved_message.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct ShapeType<'a> {
      color: &'a str,
      x: i32,
      y: i32,
      size: i32,
    }
    // this message is DataMessages serialized data withoutt encapsulation kind and encapsulation options
    let recieved_message: Vec<u8> = vec![
      0x04, 0x00, 0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x61, 0x00, 0x00, 0x00, 0x1b, 0x00, 0x00,
      0x00, 0x1e, 0x00, 0x00, 0x00,
    ];
    let recieved_message2: Vec<u8> = vec![
      0x04, 0x00, 0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x61, 0x00, 0x00, 0x00, 0x1b, 0x00, 0x00,
      0x00, 0x1e, 0x00, 0x00, 0x00,
    ];

    let deserializedMessage: ShapeType = deserialize_from_little_endian(&recieved_message).unwrap();
    println!("{:?}", deserializedMessage);

    let serializedMessage = to_bytes::<ShapeType,LittleEndian>(&deserializedMessage).unwrap();

    assert_eq!(serializedMessage, recieved_message2);
    //assert_eq!(deserializedMessage,recieved_message)
  }

  #[test]

  fn CDR_Deserialization_custom_data_message_from_ROS_and_wireshark() {
    // IDL of messsage
    //float64 x
    //float64 y
    //float64 heading
    //float64 v_x
    //float64 v_y
    //float64 kappa
    //string test

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct messageType<'a> {
      x: f64,
      y: f64,
      heading: f64,
      v_x: f64,
      v_y: f64,
      kappa: f64,
      test: &'a str,
    }

    let recieved_message_le: Vec<u8> = vec![
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1f, 0x85, 0xeb, 0x51, 0xb8,
      0x1e, 0xd5, 0x3f, 0x0a, 0x00, 0x00, 0x00, 0x54, 0x6f, 0x69, 0x6d, 0x69, 0x69, 0x6b, 0x6f,
      0x3f, 0x00, 0x00, 0x00,
    ];

    let value: messageType = deserialize_from_little_endian(&recieved_message_le).unwrap();
    println!("{:?}", value);
    assert_eq!(value.test, "Toimiiko?");
  }

  #[test]

  fn CDR_Deserialization_custom_type() {
    // IDL Definition of message:
    /*struct InterestingMessage
    {
    string unboundedString;
      long x;
      long y;
      long shapesize;
      float liuku;
      double tuplaliuku;
      unsigned short kolmeLyhytta [3];
      short neljaLyhytta [4];
      sequence<boolean> totuusarvoja;
      sequence<octet,3> kolmeTavua;
    };
    */

    // values put to serilization message with eprosima fastbuffers
    /*
      ser_var.unboundedString("tassa on aika pitka teksti");
      ser_var.x(1);
      ser_var.x(2);
      ser_var.y(-3);
      ser_var.shapesize(-4);
      ser_var.liuku(5.5);
      ser_var.tuplaliuku(-6.6);
      std::array<uint16_t, 3> foo  = {1,2,3};
      ser_var.kolmeLyhytta(foo);
      std::array<int16_t, 4>  faa = {1,-2,-3,4};
      ser_var.neljaLyhytta(faa);
      ser_var.totuusarvoja({true,false,true});
      ser_var.kolmeTavua({23,0,2});
    */

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct InterestingMessage<'a> {
      unboundedString: &'a str,
      x: i32,
      y: i32,
      shapesize: i32,
      liuku: f32,
      tuplaliuku: f64,
      kolmeLyhytta: [u16; 3],
      neljaLyhytta: [i16; 4],
      totuusarvoja: Vec<bool>,
      kolmeTavua: Vec<u8>,
    };

    let value = InterestingMessage {
      unboundedString: "Tassa on aika pitka teksti",
      x: 2,
      y: -3,
      shapesize: -4,
      liuku: 5.5,
      tuplaliuku: -6.6,
      kolmeLyhytta: [1, 2, 3],
      neljaLyhytta: [1, -2, -3, 4],
      totuusarvoja: vec![true, false, true],
      kolmeTavua: [23, 0, 2].to_vec(),
    };

    let DATA: Vec<u8> = vec![
      0x1b, 0x00, 0x00, 0x00, 0x54, 0x61, 0x73, 0x73, 0x61, 0x20, 0x6f, 0x6e, 0x20, 0x61, 0x69,
      0x6b, 0x61, 0x20, 0x70, 0x69, 0x74, 0x6b, 0x61, 0x20, 0x74, 0x65, 0x6b, 0x73, 0x74, 0x69,
      0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0xfd, 0xff, 0xff, 0xff, 0xfc, 0xff, 0xff, 0xff, 0x00,
      0x00, 0xb0, 0x40, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x1a, 0xc0, 0x01, 0x00, 0x02, 0x00,
      0x03, 0x00, 0x01, 0x00, 0xfe, 0xff, 0xfd, 0xff, 0x04, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
      0x00, 0x01, 0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x17, 0x00, 0x02,
    ];

    let serializationResult_le = to_bytes::<InterestingMessage,LittleEndian>(&value).unwrap();

    assert_eq!(serializationResult_le, DATA);
    println!("serialization success!");
    let deserializationResult: InterestingMessage = deserialize_from_little_endian(&DATA).unwrap();

    println!("{:?}", deserializationResult);
  }

  #[test]
  fn CDR_Deserialization_u8() {
    let numberU8: u8 = 35;
    let serializedNumberU8 = to_bytes::<u8,LittleEndian>(&numberU8).unwrap();
    let deSerializedNmberU8 = deserialize_from_little_endian(&serializedNumberU8).unwrap();
    assert_eq!(numberU8, deSerializedNmberU8);
    assert_eq!(deSerializedNmberU8, 35u8)
  }

  #[test]
  fn CDR_Deserialization_u16() {
    let numberU16: u16 = 35;
    let serializedNumberu16 = to_bytes::<u16,LittleEndian>(&numberU16).unwrap();
    let deSerializedNmberU16 = deserialize_from_little_endian(&serializedNumberu16).unwrap();
    assert_eq!(numberU16, deSerializedNmberU16);
    assert_eq!(deSerializedNmberU16, 35u16);
  }

  #[test]
  fn CDR_Deserialization_u32() {
    let numberU32: u32 = 352323;
    let serializedNumberu32 = to_bytes::<u32,LittleEndian>(&numberU32).unwrap();
    let deSerializedNmberU32 = deserialize_from_little_endian(&serializedNumberu32).unwrap();
    assert_eq!(numberU32, deSerializedNmberU32);
    assert_eq!(deSerializedNmberU32, 352323);
  }

  #[test]
  fn CDR_Deserialization_u64() {
    let numberU64: u64 = 352323232;
    let serializedNumberu64 = to_bytes::<u64,LittleEndian>(&numberU64).unwrap();
    let deSerializedNmberU64 = deserialize_from_little_endian(&serializedNumberu64).unwrap();
    assert_eq!(numberU64, deSerializedNmberU64);
    assert_eq!(deSerializedNmberU64, 352323232);
  }

  #[test]
  fn CDR_Deserialization_i8() {
    let numberi8: i8 = -3;
    let serializedNumberi8 = to_bytes::<i8,LittleEndian>(&numberi8).unwrap();
    let deSerializedNmberi8 = deserialize_from_little_endian(&serializedNumberi8).unwrap();
    assert_eq!(numberi8, deSerializedNmberi8);
    assert_eq!(deSerializedNmberi8, -3i8);
    assert_eq!(numberi8, -3i8);
  }

  #[test]
  fn CDR_Deserialization_i16() {
    let numberi16: i16 = -3;
    let serializedNumberi16 = to_bytes::<i16,LittleEndian>(&numberi16).unwrap();
    let deSerializedNmberi16 = deserialize_from_little_endian(&serializedNumberi16).unwrap();
    assert_eq!(numberi16, deSerializedNmberi16);
    assert_eq!(deSerializedNmberi16, -3i16);
    assert_eq!(numberi16, -3i16);
  }
  #[test]
  fn CDR_Deserialization_i32() {
    let numberi32: i32 = -323232;
    let serializedNumberi32 = to_bytes::<i32,LittleEndian>(&numberi32).unwrap();
    let deSerializedNmberi32 = deserialize_from_little_endian(&serializedNumberi32).unwrap();
    assert_eq!(numberi32, deSerializedNmberi32);
    assert_eq!(deSerializedNmberi32, -323232);
    assert_eq!(numberi32, -323232);
  }
  #[test]
  fn CDR_Deserialization_i64() {
    let numberi64: i64 = -3232323434;
    let serializedNumberi64 = to_bytes::<i64,LittleEndian>(&numberi64).unwrap();
    let deSerializedNmberi64 = deserialize_from_little_endian(&serializedNumberi64).unwrap();
    assert_eq!(numberi64, deSerializedNmberi64);
    assert_eq!(deSerializedNmberi64, -3232323434);
    assert_eq!(numberi64, -3232323434);
  }

  #[test]
  fn CDR_Deserialization_Boolean() {
    let boolean: bool = true;
    let serialized = to_bytes::<bool,LittleEndian>(&boolean).unwrap();
    let deserialized: bool = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(deserialized, boolean);
    assert_eq!(boolean, true);
    assert_eq!(deserialized, true);

    let booleanF: bool = false;
    let serializedF = to_bytes::<bool,LittleEndian>(&booleanF).unwrap();
    let deserializedF: bool = deserialize_from_little_endian(&serializedF).unwrap();
    assert_eq!(deserializedF, booleanF);
    assert_eq!(booleanF, false);
    assert_eq!(deserializedF, false);
  }

  #[test]
  fn CDR_Deserialization_f32() {
    let number: f32 = 2.35f32;
    let serialized = to_bytes::<f32,LittleEndian>(&number).unwrap();
    let deserialized: f32 = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(number, deserialized);
    assert_eq!(number, 2.35f32);
    assert_eq!(deserialized, 2.35f32);
  }

  #[test]
  fn CDR_Deserialization_f64() {
    let number: f64 = 278.35f64;
    let serialized = to_bytes::<f64,LittleEndian>(&number).unwrap();
    let deserialized: f64 = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(number, deserialized);
    assert_eq!(number, 278.35f64);
    assert_eq!(deserialized, 278.35f64);
  }
  #[test]
  fn CDR_Deserialization_char() {
    let c: char = 'a';
    let serialized = to_bytes::<char,LittleEndian>(&c).unwrap();
    let deserialized: char = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(c, deserialized);
    assert_eq!(c, 'a');
    assert_eq!(deserialized, 'a');
  }

  #[test]
  fn CDR_Deserialization_str() {
    let c: &str = "BLUE";
    let serialized = to_bytes::<&str,LittleEndian>(&c).unwrap();
    let deserialized: &str = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(c, deserialized);
    assert_eq!(c, "BLUE");
    assert_eq!(deserialized, "BLUE");
  }

  #[test]

  fn CDR_Deserialization_string() {
    let c: String = String::from("BLUE");
    let serialized = to_bytes::<String,LittleEndian>(&c).unwrap();
    let deserialized: String = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(c, deserialized);
    assert_eq!(c, String::from("BLUE"));
    assert_eq!(deserialized, String::from("BLUE"));
  }

  /*
  #[test]
  fn CDR_Deserialization_bytes(){
    let mut buf = B::with_capacity(1024);
    buf.put(&b"hello world"[..]);
    buf.put_u16(1234);

    let ubuf = buf.into(u8);
    let mut serialized = to_little_endian_binary(&ubuf).unwrap();
    let deserialized : Vec<u8> = deserialize_from_little_endian(&mut serialized).unwrap();

  }
  */

  #[test]
  fn CDR_Deserialization_seq() {
    let sequence: Vec<i32> = [1i32, -2i32, 3i32].to_vec();
    let serialized = to_bytes::<Vec<i32>,LittleEndian>(&sequence).unwrap();
    let deserialized: Vec<i32> = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(sequence, deserialized);
    assert_eq!(sequence, [1i32, -2i32, 3i32].to_vec());
    assert_eq!(deserialized, [1i32, -2i32, 3i32].to_vec());
  }

  #[test]
  fn CDR_Deserialization_unknown_type() {
    let sequence: Vec<i32> = [1i32, -2i32, 3i32, -4i32].to_vec();
    let _serialized = to_bytes::<Vec<i32>,LittleEndian>(&sequence).unwrap();
    //let TargetType: Vec<i32>;
    //TargetType = 2;

    fn type_of<T>(_: T) -> &'static str {
      type_name::<T>()
    }
    //let tt = type_of(TargetType);

    //let t = type_name_of_val(TargetType);
    //let deserialized :&str  = deserialize_from_little_endian(&mut serialized).unwrap();
  }
}
