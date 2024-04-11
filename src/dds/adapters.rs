/// DeserializerAdapter is used to fit serde Deserializer implementations and
/// DataReader together.
///
/// DataReader cannot assume a specific serialization
/// format, so it needs to be given as a parameter.
///
/// for WITH_KEY topics, we need to be able to (de)serialize the key in addition
/// to data.
pub mod no_key {
  use bytes::Bytes;

  use crate::RepresentationIdentifier;

  /// trait for connecting a Deserializer implementation and DataReader
  /// together - no_key version.
  ///
  /// Deserialization is done in two steps: decode and transform.
  /// The basic pipeline is:
  /// 
  /// byte slice → decode → value of type `Self::Deserialized` → call
  /// [`Self::transform_deserialized`] → value of type `D`
  ///
  /// The transform step may be an identity function, which means that
  /// `Self::Deserialized` and `D` are the same.
  ///
  /// The decoding step can be parameterized at run time, if the decoder is defined suitaby.
  /// If there is no need for such parameterization, use subtrait `DefaultDecoder`.
  pub trait DeserializerAdapter<D> {
    /// The error type returned when decoding fails.
    type Error: std::error::Error;

    /// Type after deserialization.
    ///
    /// The adapter might apply additional operations or wrapper types to the
    /// deserialized value, so this type might be different from `D`. 
    ///
    type Deserialized;

    /// Which data representations can the DeserializerAdapter read?
    /// See RTPS specification Section 10 and Table 10.3
    fn supported_encodings() -> &'static [RepresentationIdentifier];

    /// Transform the `Self::Deserialized` type returned by the decoder into a
    /// value of type `D`.
    ///
    /// If [`Self::Deserialized`] is set to `D`, this method can be the identity
    /// function.
    fn transform_deserialized(deserialized: Self::Deserialized) -> D;

    /// Deserialize data from bytes to an object using the given decoder.
    ///
    /// `encoding` must be something given by `supported_encodings()`, or
    /// implementation may fail with Err or `panic!()`.
    fn from_bytes_with<S>(
      input_bytes: &[u8],
      encoding: RepresentationIdentifier,
      decoder: S,
    ) -> Result<D, S::Error>
    where
      S: Decode<Self::Deserialized>,
    {
      decoder
        .decode_bytes(input_bytes, encoding)
        .map(Self::transform_deserialized)
    }

    /// Deserialize data from bytes to an object.
    /// `encoding` must be something given by `supported_encodings()`, or
    /// implementation may fail with Err or `panic!()`.
    ///
    /// Only usable if the adapter has a default decoder.
    fn from_bytes(input_bytes: &[u8], encoding: RepresentationIdentifier) -> Result<D, Self::Error>
    where
      Self: DefaultDecoder<D>,
    {
      Self::from_bytes_with(input_bytes, encoding, Self::DECODER)
    }

    /// Decode from a slice of [`Bytes`] using the given decoder.
    ///
    /// This method has a default implementation, but the default will make a
    /// copy of all the input data in memory and then call
    /// [`Self::from_bytes_with`].
    // In order to avoid the copy, implement also this method.
    fn from_vec_bytes_with<S>(
      input_vec_bytes: &[Bytes],
      encoding: RepresentationIdentifier,
      decoder: S,
    ) -> Result<D, Self::Error>
    where
      S: Decode<Self::Deserialized, Error = Self::Error>,
    {
      let total_len = input_vec_bytes.iter().map(Bytes::len).sum();
      let mut total_payload = Vec::with_capacity(total_len);
      for iv in input_vec_bytes {
        total_payload.extend(iv);
      }
      Self::from_bytes_with(&total_payload, encoding, decoder)
    }

    /// Decode from a slice of [`Bytes`].
    ///
    /// Only usable if the `Self::Deserialized` type has a default decoder.
    fn from_vec_bytes(
      input_vec_bytes: &[Bytes],
      encoding: RepresentationIdentifier,
    ) -> Result<D, Self::Error>
    where
      Self: DefaultDecoder<D>,
    {
      Self::from_vec_bytes_with(input_vec_bytes, encoding, Self::DECODER)
    }
  }

  /// The `DeserializerAdapter` can be used without a decoder as there is a
  /// default one.
  pub trait DefaultDecoder<D>: DeserializerAdapter<D> {
    /// Type of the default decoder.
    ///
    /// The default decoder needs to be clonable to be usable for async stream
    /// creation (as it's needed multiple times).
    type Decoder: Decode<Self::Deserialized, Error = Self::Error> + Clone;

    /// The default decoder value.
    ///
    /// This default decoder is typically implemented by forwarding to a
    /// different trait, e.g. `serde::Deserialize`.
    const DECODER: Self::Decoder;
  }

  /// Decodes a value of type `D` from a slice of bytes and a
  /// [`RepresentationIdentifier`].
  pub trait Decode<D> {
    /// The decoding error type returned by [`Self::decode_bytes`].
    type Error: std::error::Error;

    /// Tries to decode the given byte slice to a value of type `D` using the
    /// given encoding.
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
