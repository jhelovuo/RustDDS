use std::collections::{HashMap, HashSet};

use crate::{
  security::{
    access_control::types::*,
    authentication::types::*,
    cryptographic::{builtin_types::*, cryptographic_plugin::*, types::*},
    types::*,
  },
  security_error,
};

// A struct implementing the built-in Cryptographic plugin
// See sections 8.5 and 9.5 of the Security specification (v. 1.1)
pub struct CryptographicBuiltIn {
  encode_keys_: HashMap<CryptoHandle, KeyMaterial_AES_GCM_GMAC_seq>,
  decode_keys_: HashMap<CryptoHandle, KeyMaterial_AES_GCM_GMAC_seq>,
  encrypt_options_: HashMap<CryptoHandle, EndpointSecurityAttributes>,
  participant_to_entity_info_: HashMap<ParticipantCryptoHandle, HashSet<EntityInfo>>,
  // For reverse lookups
  entity_to_participant_: HashMap<CryptoHandle, ParticipantCryptoHandle>,

  // sessions_ ?
  /// For each (local datawriter (/datareader), remote participant) pair, stores
  /// the matched remote datareader (/datawriter)
  matched_remote_entity_: HashMap<CryptoHandle, HashMap<ParticipantCryptoHandle, CryptoHandle>>,
  ///For reverse lookups,  for each remote datawriter (/datareader), stores the
  /// matched local datareader (/datawriter)
  matched_local_entity_: HashMap<CryptoHandle, CryptoHandle>,

  handle_counter_: u32,
}

impl super::Cryptographic for CryptographicBuiltIn {}

impl CryptographicBuiltIn {
  pub fn new() -> Self {
    CryptographicBuiltIn {
      encode_keys_: HashMap::new(),
      decode_keys_: HashMap::new(),
      encrypt_options_: HashMap::new(),
      participant_to_entity_info_: HashMap::new(),
      entity_to_participant_: HashMap::new(),
      matched_remote_entity_: HashMap::new(),
      matched_local_entity_: HashMap::new(),
      handle_counter_: 0,
    }
  }

  fn insert_encode_keys_(
    &mut self,
    handle: CryptoHandle,
    keys: KeyMaterial_AES_GCM_GMAC_seq,
  ) -> SecurityResult<()> {
    match self.encode_keys_.insert(handle, keys) {
      None => SecurityResult::Ok(()),
      Some(old_key_materials) => {
        self.encode_keys_.insert(handle, old_key_materials);
        SecurityResult::Err(security_error!(
          "The handle {} was already associated with encode key material",
          handle
        ))
      }
    }
  }
  fn insert_decode_keys_(
    &mut self,
    handle: CryptoHandle,
    keys: KeyMaterial_AES_GCM_GMAC_seq,
  ) -> SecurityResult<()> {
    match self.decode_keys_.insert(handle, keys) {
      None => SecurityResult::Ok(()),
      Some(old_key_materials) => {
        self.decode_keys_.insert(handle, old_key_materials);
        SecurityResult::Err(security_error!(
          "The handle {} was already associated with decode key material",
          handle
        ))
      }
    }
  }

  fn insert_entity_info_(
    &mut self,
    participant_handle: ParticipantCryptoHandle,
    entity_info: EntityInfo,
  ) {
    match self
      .participant_to_entity_info_
      .get_mut(&participant_handle)
    {
      Some(entity_set) => {
        entity_set.insert(entity_info);
      }
      None => {
        self
          .participant_to_entity_info_
          .insert(participant_handle, HashSet::from([entity_info]));
      }
    };
  }

  fn insert_attributes_(
    &mut self,
    handle: CryptoHandle,
    attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<()> {
    match self.encrypt_options_.insert(handle, attributes) {
      None => SecurityResult::Ok(()),
      Some(old_attributes) => {
        self.encrypt_options_.insert(handle, old_attributes);
        SecurityResult::Err(security_error!(
          "The handle {} was already associated with security attributes",
          handle
        ))
      }
    }
  }
}

pub mod crypto_key_exchange;
pub mod crypto_key_factory;
pub mod crypto_transform;
