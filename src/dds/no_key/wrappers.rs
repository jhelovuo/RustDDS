use std::{marker::PhantomData, ops::Deref};

use bytes::Bytes;

use crate::{
  dds::adapters::{
    *,
    no_key::{DefaultSeed, DecodeWithEncoding},
  },
  messages::submessages::submessages::RepresentationIdentifier,
  Keyed,
};

// This wrapper is used to convert NO_KEY types to WITH_KEY
// * inside the wrapper there is a NO_KEY type
// * the wrapper is good for WITH_KEY
// The wrapper introduces a dummy key of type (), which of course has an always
// known value ()
pub(crate) struct NoKeyWrapper<D> {
  pub(crate) d: D,
}

impl<D> From<D> for NoKeyWrapper<D> {
  fn from(d: D) -> Self {
    Self { d }
  }
}

// implement Deref so that &NoKeyWrapper<D> is coercible to &D
impl<D> Deref for NoKeyWrapper<D> {
  type Target = D;
  fn deref(&self) -> &Self::Target {
    &self.d
  }
}

impl<D> Keyed for NoKeyWrapper<D> {
  type K = ();
  fn key(&self) {}
}

// wrapper for SerializerAdapter
// * inside is NO_KEY
// * outside of wrapper is WITH_KEY
pub struct SAWrapper<SA> {
  no_key: PhantomData<SA>,
}

// have to implement base trait first, just trivial passthrough
impl<D, SA> no_key::SerializerAdapter<NoKeyWrapper<D>> for SAWrapper<SA>
where
  SA: no_key::SerializerAdapter<D>,
{
  type Error = SA::Error;

  fn output_encoding() -> RepresentationIdentifier {
    SA::output_encoding()
  }

  fn to_bytes(value: &NoKeyWrapper<D>) -> Result<Bytes, SA::Error> {
    SA::to_bytes(&value.d)
  }
}

// This is the point of wrapping. Implement dummy key serialization
// Of course, this is never supposed to be actually called.
impl<D, SA> with_key::SerializerAdapter<NoKeyWrapper<D>> for SAWrapper<SA>
where
  SA: no_key::SerializerAdapter<D>,
{
  fn key_to_bytes(_value: &()) -> Result<Bytes, SA::Error> {
    Ok(Bytes::new())
  }
}

// wrapper for DeserializerAdapter
// * inside is NO_KEY
// * outside of wrapper is WITH_KEY
pub struct DAWrapper<DA> {
  no_key: PhantomData<DA>,
}

// first, implement no_key DA
impl<D, DA> no_key::DeserializerAdapter<NoKeyWrapper<D>> for DAWrapper<DA>
where
  DA: no_key::DeserializerAdapter<D>,
{
  type Error = DA::Error;
  type Deserialized = DA::Deserialized;

  fn supported_encodings() -> &'static [RepresentationIdentifier] {
    DA::supported_encodings()
  }

  fn from_bytes_seed<S>(
    input_bytes: &[u8],
    encoding: RepresentationIdentifier,
    seed: S,
  ) -> Result<NoKeyWrapper<D>, DA::Error>
  where
    S: DecodeWithEncoding<Self::Deserialized, Error = Self::Error>,
  {
    DA::from_bytes_seed(input_bytes, encoding, seed).map(|d| NoKeyWrapper::<D> { d })
  }
}

impl<D, DA> DefaultSeed<NoKeyWrapper<D>> for DAWrapper<DA>
where
  DA: DefaultSeed<D>,
{
  type Seed = DA::Seed;
  const SEED: Self::Seed = DA::SEED;
}

// then, implement with_key DA
impl<D, DA> with_key::DeserializerAdapter<NoKeyWrapper<D>> for DAWrapper<DA>
where
  DA: no_key::DeserializerAdapter<D>,
{
  fn key_from_bytes(
    _input_bytes: &[u8],
    _encoding: RepresentationIdentifier,
  ) -> Result<<NoKeyWrapper<D> as Keyed>::K, DA::Error> {
    // also unreachable!() should work here, as this is not supposed to be used
    Ok(())
  }
}
