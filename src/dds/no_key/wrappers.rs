use std::{io, ops::Deref};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};

use crate::{
  dds::traits::key::Keyed, dds::traits::serde_adapters::DeserializerAdapter,
  dds::traits::serde_adapters::SerializerAdapter, serialization,
  submessages::RepresentationIdentifier,
};

// We should not expose the NoKeyWrapper type.
// TODO: Find a way how to remove the from read()'s return type and then hide this data type.
// NoKeyWrapper is defined separately for reading and writing so that
// they can require only either Serialize or Deserialize.
pub struct NoKeyWrapper<D> {
  pub d: D,
}

impl<D> NoKeyWrapper<D> {
  pub fn unwrap(self) -> D {
    self.d
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
  fn get_key(&self) -> () {
    ()
  }
}

impl<'de, D> Deserialize<'de> for NoKeyWrapper<D>
where
  D: Deserialize<'de>,
{
  fn deserialize<R>(deserializer: R) -> std::result::Result<NoKeyWrapper<D>, R::Error>
  where
    R: Deserializer<'de>,
  {
    D::deserialize(deserializer).map(|d| NoKeyWrapper::<D> { d })
  }
}

impl<D> Serialize for NoKeyWrapper<D>
where
  D: Serialize,
{
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    self.d.serialize(serializer)
  }
}

impl<D: Serialize, SA: SerializerAdapter<D>> SerializerAdapter<NoKeyWrapper<D>> for SAWrapper<SA> {
  fn output_encoding() -> RepresentationIdentifier {
    SA::output_encoding()
  }
  fn to_writer<W: io::Write>(
    writer: W,
    value: &NoKeyWrapper<D>,
  ) -> serialization::error::Result<()> {
    SA::to_writer(writer, &value.d)
  }
}

pub struct SAWrapper<SA> {
  inner: SA,
}

impl<D: DeserializeOwned, SA: DeserializerAdapter<D>> DeserializerAdapter<NoKeyWrapper<D>>
  for SAWrapper<SA>
{
  fn supported_encodings() -> &'static [RepresentationIdentifier] {
    SA::supported_encodings()
  }
  fn from_bytes<'de>(
    input_bytes: &'de [u8],
    encoding: RepresentationIdentifier,
  ) -> serialization::error::Result<NoKeyWrapper<D>> {
    SA::from_bytes(input_bytes, encoding).map(|d| NoKeyWrapper::<D> { d })
  }
}

impl<D> NoKeyWrapper<D> {}
