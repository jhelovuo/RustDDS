use std::marker::PhantomData;

use serde::{
  de::{DeserializeOwned, DeserializeSeed},
  Serialize,
};
use bytes::Bytes;
use byteorder::{BigEndian, ByteOrder, LittleEndian};

use crate::{
  dds::{
    adapters::{no_key, with_key},
    key::Keyed,
  },
  RepresentationIdentifier,
};
pub use super::*;

/// This type adapts [`CdrSerializer`] (which implements
/// [`serde::Serializer`]) to work as a [`no_key::SerializerAdapter`] and
/// [`with_key::SerializerAdapter`].
///
/// [`CdrSerializer`] cannot directly implement the trait itself, because
/// [`CdrSerializer`] has the type parameter BO open, and the adapter needs to
/// be bi-endian.
pub struct CDRSerializerAdapter<D, BO = LittleEndian>
where
  BO: ByteOrder,
{
  phantom: PhantomData<D>,
  ghost: PhantomData<BO>,
}

impl<D, BO> no_key::SerializerAdapter<D> for CDRSerializerAdapter<D, BO>
where
  D: Serialize,
  BO: ByteOrder,
{
  type Error = Error;

  fn output_encoding() -> RepresentationIdentifier {
    RepresentationIdentifier::CDR_LE
  }

  fn to_bytes(value: &D) -> Result<Bytes> {
    let size_estimate = std::mem::size_of_val(value) * 2; // TODO: crude estimate
    let mut buffer: Vec<u8> = Vec::with_capacity(size_estimate);
    to_writer::<D, BO, &mut Vec<u8>>(&mut buffer, value)?;
    Ok(Bytes::from(buffer))
  }
}

impl<D, BO> with_key::SerializerAdapter<D> for CDRSerializerAdapter<D, BO>
where
  D: Keyed + Serialize,
  <D as Keyed>::K: Serialize,
  BO: ByteOrder,
{
  fn key_to_bytes(value: &D::K) -> Result<Bytes> {
    let size_estimate = std::mem::size_of_val(value) * 2; // TODO: crude estimate
    let mut buffer: Vec<u8> = Vec::with_capacity(size_estimate);
    to_writer::<D::K, BO, &mut Vec<u8>>(&mut buffer, value)?;
    Ok(Bytes::from(buffer))
  }
}

/// This type adapts CdrDeserializer (which implements serde::Deserializer) to
/// work as a [`with_key::DeserializerAdapter`] and
/// [`no_key::DeserializerAdapter`].
///
/// CdrDeserializer cannot directly implement
/// the trait itself, because CdrDeserializer has the type parameter BO open,
/// and the adapter needs to be bi-endian.
pub struct CDRDeserializerAdapter<D> {
  phantom: PhantomData<D>,
}

const REPR_IDS: [RepresentationIdentifier; 3] = [
  RepresentationIdentifier::CDR_BE,
  RepresentationIdentifier::CDR_LE,
  RepresentationIdentifier::PL_CDR_LE,
];

impl<D> no_key::DeserializerAdapter<D> for CDRDeserializerAdapter<D> {
  type Error = Error;
  type Decoded = D;

  fn supported_encodings() -> &'static [RepresentationIdentifier] {
    &REPR_IDS
  }

  // no transform, just the identity function
  fn transform_decoded(decoded: Self::Decoded) -> D {
    decoded
  }
}

impl<D> with_key::DeserializerAdapter<D> for CDRDeserializerAdapter<D>
where
  D: Keyed + DeserializeOwned,
  <D as Keyed>::K: DeserializeOwned, // Key should do this already?
{
  type DecodedKey = D::K;

  fn transform_decoded_key(decoded_key: Self::DecodedKey) -> D::K {
    decoded_key
  }
}

/// A default decoder is available for all types that implement
/// `serde::Deserialize`.
impl<'de, D> no_key::DefaultDecoder<D> for CDRDeserializerAdapter<D>
where
  D: serde::Deserialize<'de>,
{
  type Decoder = CdrDeserializeDecoder<D>;
  const DECODER: Self::Decoder = CdrDeserializeDecoder(PhantomData);
}

impl<D> with_key::DefaultDecoder<D> for CDRDeserializerAdapter<D>
where
  D: Keyed + DeserializeOwned,
  D::K: DeserializeOwned,
{
  type Decoder = CdrDeserializeDecoder<D>;
  const DECODER: Self::Decoder = CdrDeserializeDecoder(PhantomData);
}

/// Decode type based on a `serde::Deserialize` implementation.
pub struct CdrDeserializeDecoder<D>(PhantomData<D>);

impl<'de, D> no_key::Decode<D> for CdrDeserializeDecoder<D>
where
  D: serde::Deserialize<'de>,
{
  type Error = Error;

  fn decode_bytes(self, input_bytes: &[u8], encoding: RepresentationIdentifier) -> Result<D> {
    deserialize_from_cdr_with_decoder_and_rep_id(input_bytes, encoding, PhantomData)
  }
}

impl<Dec, DecKey> with_key::Decode<Dec, DecKey> for CdrDeserializeDecoder<Dec>
where
  Dec: DeserializeOwned,
  DecKey: DeserializeOwned,
{
  fn decode_key_bytes(
    self,
    input_key_bytes: &[u8],
    encoding: RepresentationIdentifier,
  ) -> Result<DecKey> {
    deserialize_from_cdr_with_decoder_and_rep_id(input_key_bytes, encoding, PhantomData)
  }
}

impl<D> Clone for CdrDeserializeDecoder<D> {
  fn clone(&self) -> Self {
    Self(self.0)
  }
}

/// Decode type based on a `serde::de::DeserializeSeed` implementation.
#[derive(Clone)]
pub struct CdrDeserializeSeedDecoder<S, SK> {
  value_seed: S,
  key_seed: SK,
}

impl<'de, S, SK> CdrDeserializeSeedDecoder<S, SK>
where
  S: serde::de::DeserializeSeed<'de>,
  SK: serde::de::DeserializeSeed<'de>,
{
  pub fn new(value_seed: S, key_seed: SK) -> Self {
    Self {
      value_seed,
      key_seed,
    }
  }
}

/// Decode type based on a [`serde::de::DeserializeSeed`]-based decoder.
impl<'de, D, S, SK> no_key::Decode<D> for CdrDeserializeSeedDecoder<S, SK>
where
  S: serde::de::DeserializeSeed<'de, Value = D>,
{
  type Error = Error;

  fn decode_bytes(self, input_bytes: &[u8], encoding: RepresentationIdentifier) -> Result<D> {
    deserialize_from_cdr_with_decoder_and_rep_id(input_bytes, encoding, self.value_seed)
  }
}

impl<'de, Dec, DecKey, S, SK> with_key::Decode<Dec, DecKey> for CdrDeserializeSeedDecoder<S, SK>
where
  S: serde::de::DeserializeSeed<'de, Value = Dec>,
  SK: serde::de::DeserializeSeed<'de, Value = DecKey>,
{
  fn decode_key_bytes(
    self,
    input_key_bytes: &[u8],
    encoding: RepresentationIdentifier,
  ) -> Result<DecKey> {
    deserialize_from_cdr_with_decoder_and_rep_id(input_key_bytes, encoding, self.key_seed)
  }
}

/// Decode type using the given [`DeserializeSeed`]-based decoder.
///
/// Returns deserialized object. Byte count is discarded.
fn deserialize_from_cdr_with_decoder_and_rep_id<'de, S>(
  input_bytes: &[u8],
  encoding: RepresentationIdentifier,
  decoder: S,
) -> Result<S::Value>
where
  S: DeserializeSeed<'de>,
{
  match encoding {
    RepresentationIdentifier::CDR_LE | RepresentationIdentifier::PL_CDR_LE => {
      let mut deserializer = CdrDeserializer::<LittleEndian>::new(input_bytes);
      decoder.deserialize(&mut deserializer)
    }

    RepresentationIdentifier::CDR_BE | RepresentationIdentifier::PL_CDR_BE => {
      let mut deserializer = CdrDeserializer::<BigEndian>::new(input_bytes);
      decoder.deserialize(&mut deserializer)
    }

    repr_id => Err(Error::Message(format!(
      "Unknown serialization format. requested={:?}.",
      repr_id
    ))),
  }
}
