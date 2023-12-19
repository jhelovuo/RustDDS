/// DeserializerAdapter is used to fit serde Deserializer implementations and
/// DataReader together.
///
/// DataReader cannot assume a specific serialization
/// format, so it needs to be given as a parameter.
///
/// for WITH_KEY topics, we need to be able to (de)serialize the key in addition
/// to data.
pub mod no_key {
  use std::error::Error;

  use bytes::Bytes;

  use crate::RepresentationIdentifier;

  /// trait for connecting a Deserializer implementation and DataReader
  /// together - no_key version.
  pub trait DeserializerAdapter<D> {
    type Error: std::error::Error; // Error type

    /// Type after deserialization.
    ///
    /// The adapter might apply additional operations or wrapper types to the
    /// deserialized value, so this type might be different from `D`. The basic
    /// pipeline is:
    ///
    /// bytes -> deserialize -> value of type `Self::Deserialized` -> transform -> value of type `D`
    type Deserialized;

    /// Which data representations can the DeserializerAdapter read?
    /// See RTPS specification Section 10 and Table 10.3
    fn supported_encodings() -> &'static [RepresentationIdentifier];

    fn transform_deserialized(deserialized: Self::Deserialized) -> D;

    /// Deserialize data from bytes to an object using the given seed.
    ///
    /// `encoding` must be something given by `supported_encodings()`, or
    /// implementation may fail with Err or `panic!()`.
    fn from_bytes_seed<S>(
      input_bytes: &[u8],
      encoding: RepresentationIdentifier,
      seed: S,
    ) -> Result<D, S::Error>
    where
      S: Decode<Self::Deserialized>,
    {
      seed
        .decode_bytes(input_bytes, encoding)
        .map(Self::transform_deserialized)
    }

    /// Deserialize data from bytes to an object.
    /// `encoding` must be something given by `supported_encodings()`, or
    /// implementation may fail with Err or `panic!()`.
    ///
    /// Only usable if the `Self::Deserialized` type can be deserialized without a seed.
    fn from_bytes(input_bytes: &[u8], encoding: RepresentationIdentifier) -> Result<D, Self::Error>
    where
      Self: DefaultSeed<D>,
    {
      Self::from_bytes_seed(input_bytes, encoding, Self::SEED)
    }

    /// This method has a default implementation, but the default will make a
    /// copy of all the input data in memory and then call from_bytes() .
    // In order to avoid the copy, implement also this method.
    fn from_vec_bytes_seed<S>(
      input_vec_bytes: &[Bytes],
      encoding: RepresentationIdentifier,
      seed: S,
    ) -> Result<D, Self::Error>
    where
      S: Decode<Self::Deserialized, Error = Self::Error>,
    {
      let total_len = input_vec_bytes.iter().map(Bytes::len).sum();
      let mut total_payload = Vec::with_capacity(total_len);
      for iv in input_vec_bytes {
        total_payload.extend(iv);
      }
      Self::from_bytes_seed(&total_payload, encoding, seed)
    }

    /// Deserialize from a vector of `Bytes`.
    ///
    /// Only usable if the `Self::Deserialized` type can be deserialized without a seed.
    fn from_vec_bytes(
      input_vec_bytes: &[Bytes],
      encoding: RepresentationIdentifier,
    ) -> Result<D, Self::Error>
    where
      Self: DefaultSeed<D>,
    {
      Self::from_vec_bytes_seed(input_vec_bytes, encoding, Self::SEED)
    }
  }

  pub trait DefaultSeed<D>: DeserializerAdapter<D> {
    type Seed: Decode<Self::Deserialized, Error = Self::Error> + Clone;
    const SEED: Self::Seed;
  }

  pub trait Decode<D> {
    type Error: Error;

    fn decode_bytes(
      self,
      input_bytes: &[u8],
      encoding: RepresentationIdentifier,
    ) -> Result<D, Self::Error>;
  }

  /// trait for connecting a Serializer implementation and DataWriter
  /// together - no_key version.
  pub trait SerializerAdapter<D> {
    type Error: std::error::Error; // Error type

    // what encoding do we produce?
    fn output_encoding() -> RepresentationIdentifier;

    fn to_bytes(value: &D) -> Result<Bytes, Self::Error>;
  }
}

pub mod with_key {
  use bytes::Bytes;

  use crate::{Keyed, RepresentationIdentifier};
  use super::no_key;

  /// trait for connecting a Deserializer implementation and DataReader
  /// together - with_key version.
  pub trait DeserializerAdapter<D>: no_key::DeserializerAdapter<D>
  where
    D: Keyed,
  {
    /// Deserialize a key `D::K` from bytes.
    fn key_from_bytes(
      input_bytes: &[u8],
      encoding: RepresentationIdentifier,
    ) -> Result<D::K, Self::Error>;
  }

  /// trait for connecting a Serializer implementation and DataWriter
  /// together - with_key version.
  pub trait SerializerAdapter<D>: no_key::SerializerAdapter<D>
  where
    D: Keyed,
  {
    /// serialize a key `D::K` to Bytes.
    fn key_to_bytes(value: &D::K) -> Result<Bytes, Self::Error>;
  }
}
