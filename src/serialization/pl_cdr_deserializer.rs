use std::marker::PhantomData;

use serde::de::DeserializeOwned;

//use byteorder::{ByteOrder, LittleEndian, BigEndian};
use crate::{
  dds::traits::{
    serde_adapters::{no_key, with_key},
    Keyed,
  },
  messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier,
  serialization::error::{Error, Result},
};

// This is to be implemented by all Discovery message types.
// .. likely it is not useful for others.
pub trait PlCdrDeserialize {
  // encoding must be either PL_CDR_LE or PL_CDR_BE
  fn from_pl_cdr_bytes(input_bytes: &[u8], encoding: RepresentationIdentifier) -> Result<Self>
  where
    Self: Sized;
}

pub struct PlCdrDeserializerAdapter<D> {
  phantom: PhantomData<D>,
}

const REPR_IDS: [RepresentationIdentifier; 2] = [
  // PL_CDR_* are expected
  RepresentationIdentifier::PL_CDR_BE,
  RepresentationIdentifier::PL_CDR_LE,
];

impl<D> no_key::DeserializerAdapter<D> for PlCdrDeserializerAdapter<D>
where
  D: DeserializeOwned + PlCdrDeserialize,
{
  fn supported_encodings() -> &'static [RepresentationIdentifier] {
    &REPR_IDS
  }

  fn from_bytes(input_bytes: &[u8], encoding: RepresentationIdentifier) -> Result<D> {
    match encoding {
      RepresentationIdentifier::PL_CDR_LE | RepresentationIdentifier::PL_CDR_BE => {
        D::from_pl_cdr_bytes(input_bytes, encoding)
      }
      repr_id => Err(Error::Message(format!(
        "Unknown representation identifier {:?}",
        repr_id
      ))),
    }
  }
}

impl<D> with_key::DeserializerAdapter<D> for PlCdrDeserializerAdapter<D>
where
  D: Keyed + DeserializeOwned + PlCdrDeserialize,
  <D as Keyed>::K: DeserializeOwned + PlCdrDeserialize,
{
  fn key_from_bytes(input_bytes: &[u8], encoding: RepresentationIdentifier) -> Result<D::K> {
    match encoding {
      RepresentationIdentifier::PL_CDR_LE | RepresentationIdentifier::PL_CDR_BE => {
        <D::K>::from_pl_cdr_bytes(input_bytes, encoding)
      }
      repr_id => Err(Error::Message(format!(
        "Unknown representation identifier {:?}",
        repr_id
      ))),
    }
  }
}
