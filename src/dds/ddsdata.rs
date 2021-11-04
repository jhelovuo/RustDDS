#[cfg(test)]
use bytes::Bytes;

use crate::{
  dds::traits::key::KeyHash,
  messages::submessages::submessage_elements::serialized_payload::SerializedPayload,
};
//use crate::messages::submessages::submessages::RepresentationIdentifier;
use crate::structure::cache_change::ChangeKind;

// DDSData represets a serialized data sample with metadata

#[derive(Debug, PartialEq, Clone)]

// Contents of a DATA submessage or several DATAFRAG submessages. This is either
// a new sample, or key, or a key hash. The latter two are used to indicate
// dispose or unregister.
pub enum DDSData {
  Data {
    serialized_payload: SerializedPayload,
  },
  // DataFrags {
  //   // Each DATAFRAG specifies RepresentationIdentifier, but we assume they are the same.
  //   // Otherwise, decoding would be exceedingly confusing.
  //   representation_identifier: RepresentationIdentifier,
  //   // The payload is stored as a Vec of Bytes buffers.
  //   // Deserializer should concateneate these and deserialize.
  //   bytes_frags: Vec<Bytes>,
  // },
  DisposeByKey {
    change_kind: ChangeKind,
    key: SerializedPayload,
  },
  DisposeByKeyHash {
    change_kind: ChangeKind,
    key_hash: KeyHash,
  },
}

impl DDSData {
  pub fn new(serialized_payload: SerializedPayload) -> DDSData {
    DDSData::Data { serialized_payload }
  }
  pub fn new_disposed_by_key(change_kind: ChangeKind, key: SerializedPayload) -> DDSData {
    DDSData::DisposeByKey { change_kind, key }
  }

  pub fn new_disposed_by_key_hash(change_kind: ChangeKind, key_hash: KeyHash) -> DDSData {
    DDSData::DisposeByKeyHash {
      change_kind,
      key_hash,
    }
  }

  pub fn change_kind(&self) -> ChangeKind {
    match self {
      DDSData::Data {..} /*| DDSData::DataFrags {..}*/ => ChangeKind::Alive,
      DDSData::DisposeByKey { change_kind, ..} => *change_kind,
      DDSData::DisposeByKeyHash { change_kind, .. } => *change_kind,
    }
  }

  // pub fn serialized_payload(&self) -> Option<&SerializedPayload> {
  //   match &self {
  //     DDSData::Data { serialized_payload } => Some( serialized_payload ),
  //     DDSData::DisposeByKey { key , ..} => Some( key ),
  //     DDSData::DisposeByKeyHash {..} => None,
  //   }
  // }

  #[cfg(test)]
  pub fn data(&self) -> Option<Bytes> {
    match &self {
      DDSData::Data { serialized_payload } => Some(serialized_payload.value.clone()),
      // DDSData::DataFrags { _representation_identifier, bytes_frags } =>
      //   Some(   ) ,
      DDSData::DisposeByKey { key, .. } => Some(key.value.clone()),
      DDSData::DisposeByKeyHash { .. } => None,
    }
  }
}
