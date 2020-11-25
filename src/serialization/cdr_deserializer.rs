use byteorder::{ByteOrder, LittleEndian, BigEndian, ReadBytesExt};
use std::marker::PhantomData;
//use serde::Deserialize;
//use serde::Deserializer;
use serde::{
  de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor, DeserializeOwned,
  },
};

use paste::paste;

use crate::serialization::error::Error;
use crate::serialization::error::Result;
use crate::dds::traits::serde_adapters::DeserializerAdapter;

use crate::messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier;

/// This type adapts CDR_deserializer (which implements serde::Deserializer) to work as a
/// [`DeserializerAdapter`]. CDR_deserializer cannot directly implement the trait itself, because
/// CDR_deserializer has the type parameter BO open, and the adapter needs to be bi-endian.
///
/// [`DeserializerAdapter`]: ../dds/traits/serde_adapters/trait.DeserializerAdapter.html
pub struct CDRDeserializerAdapter<D> {
  phantom: PhantomData<D>,
  // no-one home
}

const repr_ids: [RepresentationIdentifier; 3] = [
  RepresentationIdentifier::CDR_BE,
  RepresentationIdentifier::CDR_LE,
  RepresentationIdentifier::PL_CDR_LE,
];

impl<D> DeserializerAdapter<D> for CDRDeserializerAdapter<D>
where
  D: DeserializeOwned,
{
  fn supported_encodings() -> &'static [RepresentationIdentifier] {
    &repr_ids
  }

  fn from_bytes<'de>(input_bytes: &'de [u8], encoding: RepresentationIdentifier) -> Result<D> {
    match encoding {
      RepresentationIdentifier::CDR_LE | RepresentationIdentifier::PL_CDR_LE => {
        deserialize_from_little_endian(input_bytes)
      }
      RepresentationIdentifier::CDR_BE => deserialize_from_big_endian(input_bytes),
      repr_id => Err(Error::Message(format!(
        "Unknown representaiton identifier {}.",
        u16::from(repr_id)
      ))),
    }
  }
}

/// CDR deserializer.
/// Input is from &[u8], since we expect to have the data in contiguous memory buffers.
pub struct CDR_deserializer<'de, BO> {
  phantom: PhantomData<BO>, // This field exists only to provide use for BO. See PhantomData docs.
  input: &'de [u8],         // We borrow the input data, therefore we carry lifetime 'de all around.
  serializedDataCount: usize, // This is to keep track of CDR data alignment requirements.
}

impl<'de, BO> CDR_deserializer<'de, BO>
where
  BO: ByteOrder,
{
  pub fn new_little_endian(input: &[u8]) -> CDR_deserializer<LittleEndian> {
    CDR_deserializer::<LittleEndian>::new(input)
  }

  pub fn new_big_endian(input: &[u8]) -> CDR_deserializer<BigEndian> {
    CDR_deserializer::<BigEndian>::new(input)
  }

  pub fn new(input: &'de [u8]) -> CDR_deserializer<'de, BO> {
    CDR_deserializer::<BO> {
      phantom: PhantomData,
      input,
      serializedDataCount: 0,
    }
  }

  /// Read the first bytes in the input.
  fn next_bytes(&mut self, count: usize) -> Result<&[u8]> {
    if count <= self.input.len() {
      let (head, tail) = self.input.split_at(count);
      self.input = tail;
      self.serializedDataCount = self.serializedDataCount + count;
      Ok(head)
    } else {
      Err(Error::Eof)
    }
  }

  /// consume and discard bytes
  fn remove_bytes_from_input(&mut self, count: usize) -> Result<()> {
    let _pad = self.next_bytes(count)?;
    Ok(())
  }

  // Look at the first byte in the input without consuming it.
  fn peek_byte(&mut self) -> Result<u8> {
    self.input.first().ok_or(Error::Eof).map(|b| *b)
  }

  fn check_if_bytes_left(&mut self) -> bool {
    self.input.len() > 0
  }

  fn calculate_padding_count_from_written_bytes_and_remove(
    &mut self,
    typeOctetAligment: usize,
  ) -> Result<()> {
    let modulo = self.serializedDataCount % typeOctetAligment;
    if modulo != 0 {
      let padding = typeOctetAligment - modulo;
      self.remove_bytes_from_input(padding)
    } else {
      Ok(())
    }
  }
}

pub fn deserialize_from_little_endian<'a, T>(s: &'a [u8]) -> Result<T>
where
  T: DeserializeOwned,
{
  let mut deserializer = CDR_deserializer::<LittleEndian>::new(s);
  T::deserialize(&mut deserializer)
  // if deserializer.input.is_empty() {
  //   Ok(t)
  // } else {
  //   Err(Error::TrailingCharacters(deserializer.input.to_vec()))
  // }
}

pub fn deserialize_from_big_endian<'a, T>(s: &'a [u8]) -> Result<T>
where
  T: DeserializeOwned,
{
  let mut deserializer = CDR_deserializer::<BigEndian>::new(s);
  T::deserialize(&mut deserializer)
  // if deserializer.input.is_empty() {
  //   Ok(t)
  // } else {
  //   Err(Error::TrailingCharacters(deserializer.input.to_vec()))
  // }
}

/// macro for writing primitive number deserializers. Rust does not allow declaring a macro
/// inside impl block, so it is here.
macro_rules! deserialize_multibyte_number {
  ($num_type:ident) => {
    paste! {
      fn [<deserialize_ $num_type>]<V>(self, visitor: V) -> Result<V::Value>
      where
        V: Visitor<'de>,
      {
        const size :usize = std::mem::size_of::<$num_type>();
        assert!(size > 1, "multibyte means size must be > 1");
        self.calculate_padding_count_from_written_bytes_and_remove(size)?;
        visitor.[<visit_ $num_type>](
          self.next_bytes(size)?.[<read_ $num_type>]::<BO>().unwrap() )
      }
    }
  };
}

impl<'de, 'a, BO> de::Deserializer<'de> for &'a mut CDR_deserializer<'de, BO>
where
  BO: ByteOrder,
{
  type Error = Error;

  /// CDR serialization is not a self-describing data format, so we cannot implement this.
  fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  //15.3.1.5 Boolean
  //  Boolean values are encoded as single octets, where TRUE is the value 1, and FALSE as 0.
  fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    match self.next_bytes(1)?.first().unwrap() {
      0 => visitor.visit_bool(false),
      1 => visitor.visit_bool(true),
      x => Err(Error::BadBoolean(*x)),
    }
  }

  deserialize_multibyte_number!(i16);
  deserialize_multibyte_number!(i32);
  deserialize_multibyte_number!(i64);

  deserialize_multibyte_number!(u16);
  deserialize_multibyte_number!(u32);
  deserialize_multibyte_number!(u64);

  deserialize_multibyte_number!(f32);
  deserialize_multibyte_number!(f64);

  // Single-byte numbers have a bit simpler logic: No alignment, no endianness.
  fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_i8(self.next_bytes(1)?.read_i8().unwrap())
  }

  fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_u8(self.next_bytes(1)?.read_u8().unwrap())
  }

  /// Since this is Rust, a char is 32-bit Unicode codepoint.
  fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(4)?;
    let codepoint = self.next_bytes(4)?.read_u32::<BO>().unwrap();
    // TODO: Temporary workaround until std::char::from_u32() makes it into stable
    // matched value should be char::from_u32( codepoint )
    match Some(codepoint as u8 as char) {
      Some(c) => visitor.visit_char(c),
      None => Err(Error::BadChar(codepoint)),
    }
  }

  fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    // read string length
    self.calculate_padding_count_from_written_bytes_and_remove(4)?;
    let bytes_len = self.next_bytes(4)?.read_u32::<BO>().unwrap() as usize;

    let bytes = self.next_bytes(bytes_len)?; // length includes null terminator

    let bytes_without_null = &bytes[0..bytes.len() - 1];

    match std::str::from_utf8(bytes_without_null) {
      Ok(s) => visitor.visit_str(s),
      Err(utf8_err) => Err(Error::BadString(utf8_err)),
    }
  }

  fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_str(visitor)
  }

  // Byte strings

  fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_seq(visitor)
  }

  fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_seq(visitor)
  }

  fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(4)?;
    let enum_tag = self.next_bytes(4)?.read_u32::<BO>().unwrap();
    match enum_tag {
      0 => visitor.visit_none(),
      1 => visitor.visit_some(self),
      wtf => Err(Error::BadOption(wtf)),
    }
  }

  fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    // Unit data is not put on wire, to match behavior with cdr_serializer
    visitor.visit_unit()
  }

  fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_unit(visitor) // This means a named type, which has no data.
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
  fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(4)?;
    let element_count = self.next_bytes(4)?.read_u32::<BO>().unwrap() as usize;
    visitor.visit_seq(SequenceHelper::new(&mut self, element_count))
  }

  // if sequence is fixed length array then number of elements is not included
  fn deserialize_tuple<V>(mut self, len: usize, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_seq(SequenceHelper::new(&mut self, len))
  }

  fn deserialize_tuple_struct<V>(
    mut self,
    _name: &'static str,
    len: usize,
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_seq(SequenceHelper::new(&mut self, len))
  }

  fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(4)?;
    let element_count = self.next_bytes(4)?.read_u32::<BO>().unwrap() as usize;
    visitor.visit_map(SequenceHelper::new(&mut self, element_count))
  }

  fn deserialize_struct<V>(
    mut self,
    _name: &'static str,
    fields: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_seq(SequenceHelper::new(&mut self, fields.len()))
  }

  ///Enum values are encoded as unsigned longs. (u32)
  /// The numeric values associated with enum identifiers are determined by the order in which the identifiers appear in the enum declaration.
  /// The first enum identifier has the numeric value zero (0). Successive enum identifiers take ascending numeric values, in order of declaration from left to right.
  fn deserialize_enum<V>(
    mut self,
    _name: &'static str,
    _variants: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.calculate_padding_count_from_written_bytes_and_remove(4)?;
    visitor.visit_enum(EnumerationHelper::<BO>::new(&mut self))
  }

  /// An identifier in Serde is the type that identifies a field of a struct or
  /// the variant of an enum. In JSON, struct fields and enum variants are
  /// represented as strings. In other formats they may be represented as
  /// numeric indices.
  fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_u32(visitor)
  }

  fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_any(visitor)
  }
}

// ----------------------------------------------------------

struct EnumerationHelper<'a, 'de: 'a, BO> {
  de: &'a mut CDR_deserializer<'de, BO>,
}

impl<'a, 'de, BO> EnumerationHelper<'a, 'de, BO>
where
  BO: ByteOrder,
{
  fn new(de: &'a mut CDR_deserializer<'de, BO>) -> Self {
    EnumerationHelper::<BO> { de }
  }
}

impl<'de, 'a, BO> EnumAccess<'de> for EnumerationHelper<'a, 'de, BO>
where
  BO: ByteOrder,
{
  type Error = Error;
  type Variant = Self;

  fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
  where
    V: DeserializeSeed<'de>,
  {
    // preceeding deserialize_enum aligned to 4
    let enum_tag = self.de.next_bytes(4)?.read_u32::<BO>().unwrap();
    let val: Result<_> = seed.deserialize(enum_tag.into_deserializer());
    Ok((val?, self))
  }
}

// ----------------------------------------------------------

impl<'de, 'a, BO> VariantAccess<'de> for EnumerationHelper<'a, 'de, BO>
where
  BO: ByteOrder,
{
  type Error = Error;

  fn unit_variant(self) -> Result<()> {
    Ok(())
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

// ----------------------------------------------------------

struct SequenceHelper<'a, 'de: 'a, BO> {
  de: &'a mut CDR_deserializer<'de, BO>,
  elementCounter: usize,
  expectedCount: usize,
}

impl<'a, 'de, BO> SequenceHelper<'a, 'de, BO> {
  fn new(de: &'a mut CDR_deserializer<'de, BO>, expectedCount: usize) -> Self {
    SequenceHelper {
      de,
      elementCounter: 0,
      expectedCount: expectedCount,
    }
  }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'a, 'de, BO> SeqAccess<'de> for SequenceHelper<'a, 'de, BO>
where
  BO: ByteOrder,
{
  type Error = Error;

  fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
  where
    T: DeserializeSeed<'de>,
  {
    if self.elementCounter == self.expectedCount {
      Ok(None)
    } else {
      self.elementCounter = self.elementCounter + 1;
      seed.deserialize(&mut *self.de).map(Some)
    }
  }
}

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, 'a, BO> MapAccess<'de> for SequenceHelper<'a, 'de, BO>
where
  BO: ByteOrder,
{
  type Error = Error;

  fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
  where
    K: DeserializeSeed<'de>,
  {
    if self.elementCounter == self.expectedCount {
      Ok(None)
    } else {
      self.elementCounter = self.elementCounter + 1;
      seed.deserialize(&mut *self.de).map(Some)
    }
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
  use crate::serialization::cdr_serializer::to_bytes;
  use byteorder::{BigEndian, LittleEndian};
  use log::info;
  use crate::serialization::cdr_deserializer::deserialize_from_little_endian;
  use crate::serialization::cdr_deserializer::deserialize_from_big_endian;
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
    struct OmaTyyppi {
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
      thirteen: String,
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
      thirteen: "abc".to_string(),
    };

    let expected_serialized_result: Vec<u8> = vec![
      0x01, 0xfd, 0x00, 0x00, 0x78, 0xec, 0xff, 0xff, 0xd2, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x33, 0x33, 0xd3, 0xc0, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00,
      0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
      0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x05, 0x00, 0xfc, 0xff, 0x03, 0x00, 0xfe, 0xff,
      0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x61, 0x62, 0x63, 0x00,
    ];

    let sarjallistettu = to_bytes::<OmaTyyppi, LittleEndian>(&mikkiHiiri).unwrap();

    for x in 0..expected_serialized_result.len() {
      if expected_serialized_result[x] != sarjallistettu[x] {
        info!("index: {}", x);
      }
    }
    assert_eq!(sarjallistettu, expected_serialized_result);
    info!("serialization successfull!");

    let rakennettu: OmaTyyppi =
      deserialize_from_little_endian(&expected_serialized_result).unwrap();
    assert_eq!(rakennettu, mikkiHiiri);
    info!("deserialized: {:?}", rakennettu);
  }

  #[test]

  fn CDR_Deserialization_user_defined_data() {
    // look this example https://www.omg.org/spec/DDSI-RTPS/2.3/PDF
    //10.7 Example for User-defined Topic Data
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct ShapeType {
      color: String,
      x: i32,
      y: i32,
      size: i32,
    }

    let message = ShapeType {
      color: "BLUE".to_string(),
      x: 34,
      y: 100,
      size: 24,
    };

    let expected_serialized_result: Vec<u8> = vec![
      0x05, 0x00, 0x00, 0x00, 0x42, 0x4c, 0x55, 0x45, 0x00, 0x00, 0x00, 0x00, 0x22, 0x00, 0x00,
      0x00, 0x64, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
    ];

    let serialized = to_bytes::<ShapeType, LittleEndian>(&message).unwrap();
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

    let deserializedMessage: String = deserialize_from_little_endian(&recievedCDRString).unwrap();
    info!("{:?}", deserializedMessage);
    assert_eq!("Square", deserializedMessage);

    let recievedCDRString2: Vec<u8> = vec![
      0x0A, 0x00, 0x00, 0x00, 0x53, 0x68, 0x61, 0x70, 0x65, 0x54, 0x79, 0x70, 0x65,
      0x00, /* 0x00, 0x00, */
    ];

    let deserializedMessage2: String = deserialize_from_little_endian(&recievedCDRString2).unwrap();
    info!("{:?}", deserializedMessage2);
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
    let serializedO_le = to_bytes::<example, LittleEndian>(&o).unwrap();
    let serializedO_be = to_bytes::<example, BigEndian>(&o).unwrap();

    assert_eq!(
      serializedO_le,
      vec![0x01, 0x00, 0x00, 0x00, 0x61, 0x62, 0x63, 0x64,]
    );

    assert_eq!(
      serializedO_be,
      vec![0x00, 0x00, 0x00, 0x01, 0x61, 0x62, 0x63, 0x64,]
    );

    info!("serialization success");

    assert_eq!(deserialized_le, o);
    assert_eq!(deserialized_be, o);
    info!("deserialition success");
  }

  #[test]

  fn CDR_Deserialization_serialization_payload_shapes() {
    // This test uses wireshark captured shapes demo part of serialized message as recieved_message.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct ShapeType {
      color: String,
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
    info!("{:?}", deserializedMessage);

    let serializedMessage = to_bytes::<ShapeType, LittleEndian>(&deserializedMessage).unwrap();

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
    struct messageType {
      x: f64,
      y: f64,
      heading: f64,
      v_x: f64,
      v_y: f64,
      kappa: f64,
      test: String,
    }

    let recieved_message_le: Vec<u8> = vec![
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1f, 0x85, 0xeb, 0x51, 0xb8,
      0x1e, 0xd5, 0x3f, 0x0a, 0x00, 0x00, 0x00, 0x54, 0x6f, 0x69, 0x6d, 0x69, 0x69, 0x6b, 0x6f,
      0x3f, 0x00, //0x00, 0x00,
    ];

    let value: messageType = deserialize_from_little_endian(&recieved_message_le).unwrap();
    info!("{:?}", value);
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
    struct InterestingMessage {
      unboundedString: String,
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
      unboundedString: "Tassa on aika pitka teksti".to_string(),
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

    let serializationResult_le = to_bytes::<InterestingMessage, LittleEndian>(&value).unwrap();

    assert_eq!(serializationResult_le, DATA);
    info!("serialization success!");
    let deserializationResult: InterestingMessage = deserialize_from_little_endian(&DATA).unwrap();

    info!("{:?}", deserializationResult);
  }

  #[test]
  fn CDR_Deserialization_u8() {
    let numberU8: u8 = 35;
    let serializedNumberU8 = to_bytes::<u8, LittleEndian>(&numberU8).unwrap();
    let deSerializedNmberU8 = deserialize_from_little_endian(&serializedNumberU8).unwrap();
    assert_eq!(numberU8, deSerializedNmberU8);
    assert_eq!(deSerializedNmberU8, 35u8)
  }

  #[test]
  fn CDR_Deserialization_u16() {
    let numberU16: u16 = 35;
    let serializedNumberu16 = to_bytes::<u16, LittleEndian>(&numberU16).unwrap();
    let deSerializedNmberU16 = deserialize_from_little_endian(&serializedNumberu16).unwrap();
    assert_eq!(numberU16, deSerializedNmberU16);
    assert_eq!(deSerializedNmberU16, 35u16);
  }

  #[test]
  fn CDR_Deserialization_u32() {
    let numberU32: u32 = 352323;
    let serializedNumberu32 = to_bytes::<u32, LittleEndian>(&numberU32).unwrap();
    let deSerializedNmberU32 = deserialize_from_little_endian(&serializedNumberu32).unwrap();
    assert_eq!(numberU32, deSerializedNmberU32);
    assert_eq!(deSerializedNmberU32, 352323);
  }

  #[test]
  fn CDR_Deserialization_u64() {
    let numberU64: u64 = 352323232;
    let serializedNumberu64 = to_bytes::<u64, LittleEndian>(&numberU64).unwrap();
    let deSerializedNmberU64 = deserialize_from_little_endian(&serializedNumberu64).unwrap();
    assert_eq!(numberU64, deSerializedNmberU64);
    assert_eq!(deSerializedNmberU64, 352323232);
  }

  #[test]
  fn CDR_Deserialization_i8() {
    let numberi8: i8 = -3;
    let serializedNumberi8 = to_bytes::<i8, LittleEndian>(&numberi8).unwrap();
    let deSerializedNmberi8 = deserialize_from_little_endian(&serializedNumberi8).unwrap();
    assert_eq!(numberi8, deSerializedNmberi8);
    assert_eq!(deSerializedNmberi8, -3i8);
    assert_eq!(numberi8, -3i8);
  }

  #[test]
  fn CDR_Deserialization_i16() {
    let numberi16: i16 = -3;
    let serializedNumberi16 = to_bytes::<i16, LittleEndian>(&numberi16).unwrap();
    let deSerializedNmberi16 = deserialize_from_little_endian(&serializedNumberi16).unwrap();
    assert_eq!(numberi16, deSerializedNmberi16);
    assert_eq!(deSerializedNmberi16, -3i16);
    assert_eq!(numberi16, -3i16);
  }
  #[test]
  fn CDR_Deserialization_i32() {
    let numberi32: i32 = -323232;
    let serializedNumberi32 = to_bytes::<i32, LittleEndian>(&numberi32).unwrap();
    let deSerializedNmberi32 = deserialize_from_little_endian(&serializedNumberi32).unwrap();
    assert_eq!(numberi32, deSerializedNmberi32);
    assert_eq!(deSerializedNmberi32, -323232);
    assert_eq!(numberi32, -323232);
  }
  #[test]
  fn CDR_Deserialization_i64() {
    let numberi64: i64 = -3232323434;
    let serializedNumberi64 = to_bytes::<i64, LittleEndian>(&numberi64).unwrap();
    let deSerializedNmberi64 = deserialize_from_little_endian(&serializedNumberi64).unwrap();
    assert_eq!(numberi64, deSerializedNmberi64);
    assert_eq!(deSerializedNmberi64, -3232323434);
    assert_eq!(numberi64, -3232323434);
  }

  #[test]
  fn CDR_Deserialization_Boolean() {
    let boolean: bool = true;
    let serialized = to_bytes::<bool, LittleEndian>(&boolean).unwrap();
    let deserialized: bool = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(deserialized, boolean);
    assert_eq!(boolean, true);
    assert_eq!(deserialized, true);

    let booleanF: bool = false;
    let serializedF = to_bytes::<bool, LittleEndian>(&booleanF).unwrap();
    let deserializedF: bool = deserialize_from_little_endian(&serializedF).unwrap();
    assert_eq!(deserializedF, booleanF);
    assert_eq!(booleanF, false);
    assert_eq!(deserializedF, false);
  }

  #[test]
  fn CDR_Deserialization_f32() {
    let number: f32 = 2.35f32;
    let serialized = to_bytes::<f32, LittleEndian>(&number).unwrap();
    let deserialized: f32 = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(number, deserialized);
    assert_eq!(number, 2.35f32);
    assert_eq!(deserialized, 2.35f32);
  }

  #[test]
  fn CDR_Deserialization_f64() {
    let number: f64 = 278.35f64;
    let serialized = to_bytes::<f64, LittleEndian>(&number).unwrap();
    let deserialized: f64 = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(number, deserialized);
    assert_eq!(number, 278.35f64);
    assert_eq!(deserialized, 278.35f64);
  }
  #[test]
  fn CDR_Deserialization_char() {
    let c: char = 'a';
    let serialized = to_bytes::<char, LittleEndian>(&c).unwrap();
    let deserialized: char = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(c, deserialized);
    assert_eq!(c, 'a');
    assert_eq!(deserialized, 'a');
  }

  #[test]
  fn CDR_Deserialization_str() {
    let c: String = "BLUE".to_string();
    let serialized = to_bytes::<String, LittleEndian>(&c).unwrap();
    let deserialized: String = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(c, deserialized);
    assert_eq!(c, "BLUE");
    assert_eq!(deserialized, "BLUE");
  }

  #[test]

  fn CDR_Deserialization_string() {
    let c: String = String::from("BLUE");
    let serialized = to_bytes::<String, LittleEndian>(&c).unwrap();
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
    let serialized = to_bytes::<Vec<i32>, LittleEndian>(&sequence).unwrap();
    let deserialized: Vec<i32> = deserialize_from_little_endian(&serialized).unwrap();
    assert_eq!(sequence, deserialized);
    assert_eq!(sequence, [1i32, -2i32, 3i32].to_vec());
    assert_eq!(deserialized, [1i32, -2i32, 3i32].to_vec());
  }

  #[test]
  fn CDR_Deserialization_unknown_type() {
    let sequence: Vec<i32> = [1i32, -2i32, 3i32, -4i32].to_vec();
    let _serialized = to_bytes::<Vec<i32>, LittleEndian>(&sequence).unwrap();
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
