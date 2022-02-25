use std::{io, io::Write, marker::PhantomData};

use serde::{ser, Serialize};
use bytes::Bytes;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
#[cfg(test)]
use byteorder::BigEndian;

use crate::{
  dds::traits::{
    key::Keyed,
    serde_adapters::{no_key, with_key},
  },
  messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier,
  serialization::error::{Error, Result},
};

// This is to be implemented by all Discovery message types.
// .. likely it is not useful for others.
pub trait PlCdrSerialize {
  // encoding must be either PL_CDR_LE or PL_CDR_BE
  fn to_pl_cdr_bytes<D>(d: &D, encoding: RepresentationIdentifier) -> Result<Bytes>;
}

pub struct PlCdrSerializerAdapter<D, BO = LittleEndian> 
where
  BO: ByteOrder,
{
  phantom: PhantomData<D>,
  ghost: PhantomData<BO>,
}

impl<D,BO> no_key::SerializerAdapter<D> for PlCdrSerializerAdapter<D,BO>
where
  D: Serialize + PlCdrSerialize,
  BO: ByteOrder,
{
  fn output_encoding() -> RepresentationIdentifier {
    //TODO: This works only for BO=LittleEndian
    RepresentationIdentifier::CDR_PL_LE
  }

  fn to_bytes(value: &D) -> Result<Bytes> {
    // TODO: This works only for BO=LittleEndian
    D::to_pl_cdr_bytes(value, RepresentationIdentifier::CDR_PL_LE)
  }

}

impl<D, BO> with_key::SerializerAdapter<D> for PlCdrSerializerAdapter<D, BO>
where
  D: Keyed + Serialize + PlCdrSerialize,
  <D as Keyed>::K: Serialize + PlCdrSerialize,
  BO: ByteOrder,
{
  fn key_to_bytes(value: &D::K) -> Result<Bytes> {
    // TODO: This works only for BO=LittleEndian
    <D::K>::to_pl_cdr_bytes(value, RepresentationIdentifier::CDR_PL_LE)
  }
}
