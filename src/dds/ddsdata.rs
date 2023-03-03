use bytes::Bytes;

use crate::{
  dds::traits::key::KeyHash,
  messages::submessages::submessage_elements::serialized_payload::SerializedPayload,
};
use crate::structure::cache_change::ChangeKind;

// DDSData represets a serialized data sample with metadata

#[derive(Debug, PartialEq, Eq, Clone)]
// Contents of a DATA submessage or several DATAFRAG submessages. This is either
// a new sample, or key, or a key hash. The latter two are used to indicate
// dispose or unregister.
pub enum DDSData {
  Data {
    serialized_payload: SerializedPayload,
  },
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
  pub fn new(serialized_payload: SerializedPayload) -> Self {
    Self::Data { serialized_payload }
  }
  pub fn new_disposed_by_key(change_kind: ChangeKind, key: SerializedPayload) -> Self {
    Self::DisposeByKey { change_kind, key }
  }

  pub fn new_disposed_by_key_hash(change_kind: ChangeKind, key_hash: KeyHash) -> Self {
    Self::DisposeByKeyHash {
      change_kind,
      key_hash,
    }
  }

  #[allow(dead_code)] // Why is this not used?
  pub fn change_kind(&self) -> ChangeKind {
    match self {
      DDSData::Data {..} /*| DDSData::DataFrags {..}*/ => ChangeKind::Alive,
      DDSData::DisposeByKey { change_kind, ..} | DDSData::DisposeByKeyHash { change_kind, .. }  => *change_kind,
    }
  }

  // What is the serialized size of this?
  pub fn payload_size(&self) -> usize {
    match self {
      DDSData::Data { serialized_payload } => serialized_payload.len_serialized(),
      DDSData::DisposeByKey { key, .. } => key.len_serialized(),
      DDSData::DisposeByKeyHash { .. } => 16,
      // This is a fundamental constant of the RTPS
      // specification v2.5 Section 9.6.4.8 KeyHash (PID_KEY_HASH)
    }
  }

  #[cfg(test)]
  pub fn data(&self) -> Bytes {
    self.payload_bytes()
  }

  #[cfg(test)]
  fn payload_bytes(&self) -> Bytes {
    match &self {
      DDSData::Data { serialized_payload } => serialized_payload.value.clone(),
      DDSData::DisposeByKey { key, .. } => key.value.clone(),
      DDSData::DisposeByKeyHash { key_hash, .. } => Bytes::from(key_hash.to_vec()),
    }
  }

  pub fn bytes_slice(&self, from: usize, to: usize) -> Bytes {
    match &self {
      DDSData::Data { serialized_payload } => serialized_payload.bytes_slice(from, to),
      DDSData::DisposeByKey { key, .. } => key.bytes_slice(from, to),
      DDSData::DisposeByKeyHash { key_hash, .. } => {
        Bytes::from(key_hash.to_vec().into_boxed_slice()).slice(from..to)
      } // TODO: No bounds checkings, may crash. But this should not be used.
    }
  }
}
