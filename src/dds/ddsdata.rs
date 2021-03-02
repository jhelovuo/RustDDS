

use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::structure::cache_change::ChangeKind;

#[cfg(test)]
use bytes::Bytes;


// DDSData represets a serialized data sample with metadata

#[derive(Debug, PartialEq, Clone)]

// Contents of a DATA or severlal DATAFRAG submessage. This is either a
// new sample, or key, or a key hash. The latter two are used to indicate dispose or unregister.
pub enum DDSData {
  Data { serialized_payload: SerializedPayload } ,
  // StatusInfo is an enumeration giving reason why there is no data
  DisposeByKey { change_kind: ChangeKind, key: SerializedPayload, },
  DisposeByKeyHash { change_kind: ChangeKind, key_hash: u128, }, 
  // TODO: Key hash should be a named type. Preferebly contents should
  // be held as [u8;8] rather then u128 to avoid endianness issues.
}

impl DDSData {
  pub fn new(serialized_payload: SerializedPayload) -> DDSData {
    DDSData::Data { serialized_payload }
  }
  pub fn new_disposed_by_key(change_kind: ChangeKind, key: SerializedPayload) -> DDSData {
    DDSData::DisposeByKey { change_kind, key }
  }

  pub fn new_disposed_by_key_hash(change_kind: ChangeKind, key_hash: u128) -> DDSData {
    DDSData::DisposeByKeyHash { change_kind, key_hash }
  }

  pub fn change_kind(&self) -> ChangeKind {
    match self {
      DDSData::Data {..} => ChangeKind::ALIVE,
      DDSData::DisposeByKey { change_kind, ..} => *change_kind,
      DDSData::DisposeByKeyHash { change_kind, .. } => *change_kind,
    }
  }

  pub fn serialized_payload(&self) -> Option<&SerializedPayload> {
    match &self {
      DDSData::Data { serialized_payload } => Some( serialized_payload ),
      DDSData::DisposeByKey { key , ..} => Some( key ),
      DDSData::DisposeByKeyHash {..} => None,
    }
  }

  #[cfg(test)]
  pub fn data(&self) -> Option<Bytes> {
    match &self {
      DDSData::Data { serialized_payload } => Some( serialized_payload.value ),
      DDSData::DisposeByKey { key , ..} => Some( key.value ),
      DDSData::DisposeByKeyHash {..} => None,
    }
  }
  
}
