use byteorder::{ByteOrder, LittleEndian};
use serde::Deserialize;
use serde::de::{
  self, DeserializeSeed, EnumAccess,/* IntoDeserializer, */MapAccess,  SeqAccess, VariantAccess, Visitor,
};
use crate::serialization::error::Error;
use crate::serialization::error::Result;
//use serde::Deserializer;

pub struct DeserializerLittleEndian<'de> {
  input: &'de mut Vec<u8>,
}

impl<'de> DeserializerLittleEndian<'de> {
  pub fn deserialize_from_little_endian(input: &'de mut Vec<u8>) -> Self {
    DeserializerLittleEndian { input }
  }

  fn remove_first_byte_from_input(&mut self){
       self.input.remove(0);
  }

  fn calculate_padding_count_from_written_bytes(count: u64) -> u8{
    let modulo = count%4;
    let mut needToRemoveAmount :u8 = 0;
    if modulo == 0 {
      needToRemoveAmount = 0u8
    }else if modulo == 1 {
      needToRemoveAmount = 3u8
    }else if modulo == 2 {
      needToRemoveAmount = 2u8
    }else if modulo == 3 {
      needToRemoveAmount = 1u8
    }
    println!("need to remove pad byte count: {}", needToRemoveAmount);
    return needToRemoveAmount
  }

  fn remove_padding_bytes_from_end(&mut self, padCount :u8){
    println!("try to remove padding");
    for _a in 0..padCount{
      self.remove_first_byte_from_input();
    }
  }
}

pub fn deserialize_from_little_endian<'a, T>(s: &'a mut Vec<u8>) -> Result<T>
where
  T: Deserialize<'a>,
{
  let mut deserializer = DeserializerLittleEndian::deserialize_from_little_endian( s);
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

  fn check_if_bytes_left(&mut self) -> bool{
    let someValueFound = self.input.first().ok_or(Error::Eof);
    if someValueFound.is_ok(){
      true
    }else{
      false
    }

  }

  // Consume the first byte in the input.
  fn next_byte(&mut self) -> Result<u8> {
      let by = self.input[0];
      self.remove_first_byte_from_input();
      //self.input = &self.input[1..].to_vec();
      Ok(by)
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
    let result : bool; 
    if self.next_byte().unwrap() == 1u8{
      result = true;
    }
    else{
      result = false;
    }
    visitor.visit_bool(result)
  }

  fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let firstByte = self.next_byte().unwrap();
    let i8Byte =  firstByte as i8;
    self.remove_padding_bytes_from_end(3);
    visitor.visit_i8( i8Byte)
  }

  fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let bytes :[u8;2] = [by0,by1];
    let result : i16 = LittleEndian::read_i16(&bytes);
    self.remove_padding_bytes_from_end(2);
    visitor.visit_i16(result)
  }

  fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let by2 = self.next_byte().unwrap();
    let by3 = self.next_byte().unwrap();
    let bytes :[u8;4] = [by0,by1,by2,by3];
    let result : i32 = LittleEndian::read_i32(&bytes);
    visitor.visit_i32(result)
  }

  fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let by2 = self.next_byte().unwrap();
    let by3 = self.next_byte().unwrap();
    let by4 = self.next_byte().unwrap();
    let by5 = self.next_byte().unwrap();
    let by6 = self.next_byte().unwrap();
    let by7 = self.next_byte().unwrap();
    let bytes :[u8;8] = [by0,by1,by2,by3,by4,by5,by6,by7];
    let result : i64 = LittleEndian::read_i64(&bytes);
    visitor.visit_i64(result)
  }

  fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by = self.next_byte().unwrap();
    self.remove_padding_bytes_from_end(3);
    visitor.visit_u8(by)
  }

  fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let bytes :[u8;2] = [by0,by1];
    let result = LittleEndian::read_u16(&bytes);
    self.remove_padding_bytes_from_end(2);
    visitor.visit_u16(result)
  }

  fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let by2 = self.next_byte().unwrap();
    let by3 = self.next_byte().unwrap();
    let bytes :[u8;4] = [by0,by1,by2,by3];
    let result = LittleEndian::read_u32(&bytes);
    visitor.visit_u32(result)
  }

  fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let by2 = self.next_byte().unwrap();
    let by3 = self.next_byte().unwrap();
    let by4 = self.next_byte().unwrap();
    let by5 = self.next_byte().unwrap();
    let by6 = self.next_byte().unwrap();
    let by7 = self.next_byte().unwrap();
    let bytes :[u8;8] = [by0,by1,by2,by3,by4,by5,by6,by7];
    let result = LittleEndian::read_u64(&bytes);
    visitor.visit_u64(result)
  }

  fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let by2 = self.next_byte().unwrap();
    let by3 = self.next_byte().unwrap();
    let bytes :[u8;4] = [by0,by1,by2,by3];
    let result = LittleEndian::read_f32(&bytes);
    _visitor.visit_f32(result)
  }

  fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let by2 = self.next_byte().unwrap();
    let by3 = self.next_byte().unwrap();
    let by4 = self.next_byte().unwrap();
    let by5 = self.next_byte().unwrap();
    let by6 = self.next_byte().unwrap();
    let by7 = self.next_byte().unwrap();
    let bytes :[u8;8] = [by0,by1,by2,by3,by4,by5,by6,by7];
    let result:f64 = LittleEndian::read_f64(&bytes);
    _visitor.visit_f64(result)
  }

  fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let by0 = self.next_byte().unwrap();
    //self.remove_padding_bytes_from_end(3);
    _visitor.visit_char(by0 as char)
  }

  fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    // first is information about how long string is in bytes.
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let by2 = self.next_byte().unwrap();
    let by3 = self.next_byte().unwrap();
    let bytes :[u8;4] = [by0,by1,by2,by3];
    let stringByteCount : u32 = LittleEndian::read_u32(&bytes);
    let buildString : String;
    let mut chars: Vec<char> = [].to_vec();

    println!("trying to deserialize string of byte length: {}", stringByteCount);
  
    // last byte is always 0 and it can be ignored.
    for _byte in 0..stringByteCount - 1 {
      let c = self.next_byte().unwrap() as char;
      chars.push(c);
    }
    // here need to call next byte to remove trailing 0 from buffer.
    self.remove_first_byte_from_input();
    buildString = chars.into_iter().collect();
    
    // TODO check is this correct way to create string literals. This is propably not correct!!!
    fn string_to_static_str(s: String) -> &'static str {
      Box::leak(s.into_boxed_str())
  }
    self.remove_padding_bytes_from_end( DeserializerLittleEndian::calculate_padding_count_from_written_bytes(stringByteCount as u64));
    visitor.visit_borrowed_str(string_to_static_str(buildString))
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
    // for byte in self.input{
    //  self.next_byte();
    //}
     
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

  ///Sequences are encoded as an unsigned long value, followed by the elements of the
  //sequence. The initial unsigned long contains the number of elements in the sequence.
  //The elements of the sequence are encoded as specified for their type.
  fn deserialize_seq<V>(mut self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize seq!");
    let by0 = self.next_byte().unwrap();
    let by1 = self.next_byte().unwrap();
    let by2 = self.next_byte().unwrap();
    let by3 = self.next_byte().unwrap();
    let bytes :[u8;4] = [by0,by1,by2,by3];
    let elementCount : u32 = LittleEndian::read_u32(&bytes);
    println!("seq length: {}", elementCount);
    _visitor.visit_seq(SequenceHelper::new(&mut self))
  }

  // if sequence is fixed length array then number of elements is not included
  fn deserialize_tuple<V>(mut self, _len: usize, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize tuple!");
    visitor.visit_seq(SequenceHelper::new(&mut self))
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
    mut self,
    _name: &'static str,
    _fields: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    println!("deserialize struct! it has num of fields: {} ", _fields.len());
    for f in _fields{
      println!("field: {} ", f);
    }
    visitor.visit_seq(SequenceHelper::new(&mut self))
    //self.deserialize_tuple(_fields.len(), visitor)
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


struct SequenceHelper<'a, 'de: 'a> {
  de: &'a mut DeserializerLittleEndian<'de>,
  first: bool,
}

impl<'a, 'de> SequenceHelper<'a, 'de> {
  fn new(de: &'a mut DeserializerLittleEndian<'de>) -> Self {
    SequenceHelper {
          de,
          first: true,
      }
  }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> SeqAccess<'de> for SequenceHelper<'a, 'de> {
  type Error = Error;

  fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
  where
      T: DeserializeSeed<'de>,
  {
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
impl<'de, 'a> MapAccess<'de> for SequenceHelper<'a, 'de> {
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
  use crate::serialization::cdrSerializer::to_little_endian_binary;
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
  use serde::{Serialize, Deserialize};
  use bytes::{BytesMut, BufMut};
  use std::any::type_name;


  #[test]
  fn CDR_Deserialization_struct() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct OmaTyyppi {
      firstValue: u8,
      secondvalue: i8,
      thirdValue: i32,
      fourthValue: u64,
      fifth: bool,
      sixth: f32,
      seventh: f64,
      eigth: Vec<i32>
    }

    let mikkiHiiri = OmaTyyppi {
      firstValue: 1,
      secondvalue: -3,
      thirdValue: -5000,
      fourthValue: 90909099999999u64,
      fifth: true,
      sixth: -23.43f32,
      seventh: 3432343.3423443f64,
      eigth: vec![-1,2,-3,4]
    };

    let mut sarjallistettu = to_little_endian_binary(&mikkiHiiri).unwrap();
    println!("{:?}",sarjallistettu);
    let rakennettu: OmaTyyppi = deserialize_from_little_endian(&mut sarjallistettu).unwrap();
    assert_eq!(rakennettu, mikkiHiiri);
    println!("deserialized: {:?}", rakennettu  );
  }

  #[test]

  fn CDR_Deserialization_user_defined_data(){
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
      x:34,
      y:100,
      size:24,
    };


    let expected_serialized_result: Vec<u8> = vec![
      0x05, 0x00, 0x00, 0x00,
      0x42, 0x4c, 0x55, 0x45,
      0x00, 0x00, 0x00, 0x00,
      0x22, 0x00, 0x00, 0x00,
      0x64, 0x00, 0x00, 0x00,
      0x18, 0x00, 0x00, 0x00,
    ];

    let mut serialized = to_little_endian_binary(&message).unwrap();
    assert_eq!(serialized, expected_serialized_result);
    let deserializedMessage :ShapeType = deserialize_from_little_endian(&mut serialized).unwrap();
    assert_eq!(deserializedMessage,message)
  }

  #[test]

fn CDR_Deserialization_serialization_topic_name(){

  // look this example https://www.omg.org/spec/DDSI-RTPS/2.3/PDF
  //10.6 Example for Built-in Endpoint Data
  // this is just CRD topic name strings

  let mut recievedCDRString : Vec<u8> = vec![
    0x07, 0x00, 0x00, 0x00,
    0x053, 0x71, 0x75, 0x61,
    0x72, 0x65, 0x00, 0x00,
  ];

  let deserializedMessage : &str = deserialize_from_little_endian(&mut  recievedCDRString).unwrap();
  println!("{:?}",deserializedMessage);
  assert_eq!("Square",deserializedMessage);
  
  let mut recievedCDRString2 : Vec<u8> = vec![
    0x0A, 0x00, 0x00, 0x00,
    0x53, 0x68, 0x61, 0x70,
    0x65, 0x54, 0x79, 0x70,
    0x65, 0x00, 0x00, 0x00,
  ];

  let deserializedMessage2 : &str = deserialize_from_little_endian(&mut  recievedCDRString2).unwrap();
  println!("{:?}",deserializedMessage2);
  assert_eq!("ShapeType",deserializedMessage2);

}

#[test]
fn CDR_Deserialization_example_struct(){

  // look this example https://www.omg.org/spec/DDSI-RTPS/2.2/PDF 
  // 10.2.2 Example

  #[derive(Serialize, Deserialize, Debug, PartialEq)]
  struct example {
    a: u32,
    b: [char;4],
  }

    let o = example{
      a :1,
      b : ['a','b','c','d'],
    };

    let mut serializized: Vec<u8>= vec![
      0x01, 0x00, 0x00, 0x00,
      0x61, 0x62, 0x63, 0x64,
    ];

    let deserialized : example = deserialize_from_little_endian(&mut serializized).unwrap();
    let serializedO = to_little_endian_binary(&o).unwrap();

    assert_eq!(serializedO,vec![
      0x01, 0x00, 0x00, 0x00,
      0x61, 0x62, 0x63, 0x64,
    ]);
    println!("serialization success");

    assert_eq!(deserialized,o);
    println!("deserialition success");
    
}


  #[test]

  fn CDR_Deserialization_serialization_payload_shapes(){
    // This test uses wireshark captured shapes demo part of serialized message as recieved_message.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct ShapeType<'a> {
      color: &'a str,
      x: i32,
      y: i32,
      size: i32,
    }
    // this message is DataMessages serialized data withoutt encapsulation kind and encapsulation options
    let mut recieved_message: Vec<u8> = vec![
      0x04, 0x00, 0x00, 0x00,
      0x52, 0x45, 0x44, 0x00,
      0x61, 0x00, 0x00, 0x00,
      0x1b, 0x00, 0x00, 0x00,
      0x1e, 0x00, 0x00, 0x00,
    ];

    let deserializedMessage :ShapeType = deserialize_from_little_endian(&mut  recieved_message).unwrap();
    println!("{:?}",deserializedMessage);
    //assert_eq!(deserializedMessage,recieved_message)
    
  }
  



  #[test]
  fn CDR_Deserialization_u8() {
    let numberU8: u8 = 35;
    let mut serializedNumberU8 = to_little_endian_binary(&numberU8).unwrap();
    let deSerializedNmberU8 = deserialize_from_little_endian(&mut serializedNumberU8).unwrap();
    assert_eq!(numberU8, deSerializedNmberU8);
    assert_eq!(deSerializedNmberU8, 35u8)
  }

  #[test]
  fn CDR_Deserialization_u16(){
    let numberU16: u16 = 35;
    let mut serializedNumberu16 = to_little_endian_binary(&numberU16).unwrap();
    let deSerializedNmberU16 = deserialize_from_little_endian(&mut serializedNumberu16).unwrap();
    assert_eq!(numberU16, deSerializedNmberU16);
    assert_eq!(deSerializedNmberU16, 35u16);
  }

  #[test]
  fn CDR_Deserialization_u32(){
    let numberU32: u32 = 352323;
    let mut serializedNumberu32 = to_little_endian_binary(&numberU32).unwrap();
    let deSerializedNmberU32 = deserialize_from_little_endian(&mut serializedNumberu32).unwrap();
    assert_eq!(numberU32, deSerializedNmberU32);
    assert_eq!(deSerializedNmberU32, 352323);
  }

  #[test]
  fn CDR_Deserialization_u64(){
    let numberU64: u64 = 352323232;
    let mut serializedNumberu64 = to_little_endian_binary(&numberU64).unwrap();
    let deSerializedNmberU64 = deserialize_from_little_endian(&mut serializedNumberu64).unwrap();
    assert_eq!(numberU64, deSerializedNmberU64);
    assert_eq!(deSerializedNmberU64, 352323232);
  }

#[test]
fn CDR_Deserialization_i8(){
  let numberi8: i8 = -3;
  let mut serializedNumberi8 = to_little_endian_binary(&numberi8).unwrap();
  let deSerializedNmberi8 = deserialize_from_little_endian(&mut serializedNumberi8).unwrap();
  assert_eq!(numberi8, deSerializedNmberi8);
  assert_eq!(deSerializedNmberi8,-3i8);
  assert_eq!(numberi8,-3i8);

}

#[test]
fn CDR_Deserialization_i16(){
  let numberi16: i16 = -3;
  let mut serializedNumberi16 = to_little_endian_binary(&numberi16).unwrap();
  let deSerializedNmberi16 = deserialize_from_little_endian(&mut serializedNumberi16).unwrap();
  assert_eq!(numberi16, deSerializedNmberi16);
  assert_eq!(deSerializedNmberi16,-3i16);
  assert_eq!(numberi16,-3i16);

}
#[test]
fn CDR_Deserialization_i32(){
  let numberi32: i32 = -323232;
  let mut serializedNumberi32 = to_little_endian_binary(&numberi32).unwrap();
  let deSerializedNmberi32 = deserialize_from_little_endian(&mut serializedNumberi32).unwrap();
  assert_eq!(numberi32, deSerializedNmberi32);
  assert_eq!(deSerializedNmberi32,-323232);
  assert_eq!(numberi32,-323232);

}
#[test]
fn CDR_Deserialization_i64(){
  let numberi64: i64 = -3232323434;
  let mut serializedNumberi64 = to_little_endian_binary(&numberi64).unwrap();
  let deSerializedNmberi64 = deserialize_from_little_endian(&mut serializedNumberi64).unwrap();
  assert_eq!(numberi64, deSerializedNmberi64);
  assert_eq!(deSerializedNmberi64,-3232323434);
  assert_eq!(numberi64,-3232323434);

}

#[test]
fn CDR_Deserialization_Boolean(){
  let boolean : bool = true;
  let mut serialized = to_little_endian_binary(&boolean).unwrap();
  let deserialized :bool = deserialize_from_little_endian(&mut serialized).unwrap();
  assert_eq!(deserialized, boolean);
  assert_eq!(boolean,true);
  assert_eq!(deserialized,true);

  let booleanF : bool = false;
  let mut serializedF = to_little_endian_binary(&booleanF).unwrap();
  let deserializedF :bool = deserialize_from_little_endian(&mut serializedF).unwrap();
  assert_eq!(deserializedF, booleanF);
  assert_eq!(booleanF,false);
  assert_eq!(deserializedF,false);
}

#[test]
fn CDR_Deserialization_f32(){
  let number : f32 = 2.35f32;
  let mut serialized = to_little_endian_binary(&number).unwrap();
  let deserialized :f32 = deserialize_from_little_endian(&mut serialized).unwrap();
  assert_eq!(number, deserialized);
  assert_eq!(number,2.35f32);
  assert_eq!(deserialized,2.35f32);
}

#[test]
fn CDR_Deserialization_f64(){
  let number : f64 = 278.35f64;
  let mut serialized = to_little_endian_binary(&number).unwrap();
  let deserialized :f64 = deserialize_from_little_endian(&mut serialized).unwrap();
  assert_eq!(number, deserialized);
  assert_eq!(number, 278.35f64);
  assert_eq!(deserialized, 278.35f64);

}
#[test]
fn CDR_Deserialization_char(){
  let c : char = 'a';
  let mut serialized = to_little_endian_binary(&c).unwrap();
  let deserialized :char = deserialize_from_little_endian(&mut serialized).unwrap();
  assert_eq!(c , deserialized);
  assert_eq!(c , 'a');
  assert_eq!(deserialized, 'a');
}

#[test]
fn CDR_Deserialization_str(){
  let c : &str = "BLUE";
  let mut serialized = to_little_endian_binary(&c).unwrap();
  let deserialized : &str = deserialize_from_little_endian(&mut serialized).unwrap();
  assert_eq!(c , deserialized);
  assert_eq!(c , "BLUE");
  assert_eq!(deserialized, "BLUE");
}

#[test]

fn CDR_Deserialization_string(){
  let c : String = String::from("BLUE");
  let mut serialized = to_little_endian_binary(&c).unwrap();
  let deserialized : String = deserialize_from_little_endian(&mut serialized).unwrap();
  assert_eq!(c , deserialized);
  assert_eq!(c , String::from("BLUE"));
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
fn CDR_Deserialization_seq(){
  let sequence :Vec<i32> = [1i32,-2i32,3i32,-4i32].to_vec();
  let mut serialized = to_little_endian_binary(&sequence).unwrap();
  let deserialized :Vec<i32> = deserialize_from_little_endian(&mut serialized).unwrap();
  assert_eq!(sequence, deserialized);
  assert_eq!(sequence, [1i32,-2i32,3i32,-4i32].to_vec());
  assert_eq!(deserialized, [1i32,-2i32,3i32,-4i32].to_vec());


}

#[test]
fn CDR_Deserialization_unknown_type(){
  let sequence :Vec<i32> = [1i32,-2i32,3i32,-4i32].to_vec();
  let _serialized = to_little_endian_binary(&sequence).unwrap();
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
