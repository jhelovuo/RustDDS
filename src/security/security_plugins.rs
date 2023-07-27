use core::fmt;
use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use crate::{
  messages::submessages::{
    secure_postfix::SecurePostfix,
    secure_prefix::SecurePrefix,
    submessage::{ReaderSubmessage, WriterSubmessage},
  },
  rtps::{Message, Submessage},
  security_error,
  structure::guid::GuidPrefix,
  GUID,
};
use super::{
  cryptographic::{
    DatareaderCryptoHandle, DatawriterCryptoHandle, EncodedSubmessage, EntityCryptoHandle,
    ParticipantCryptoHandle, SecureSubmessageCategory,
  },
  AccessControl, Authentication, Cryptographic, SecurityError, SecurityResult,
};

pub(crate) struct SecurityPlugins {
  pub auth: Box<dyn Authentication>,
  pub access: Box<dyn AccessControl>,
  crypto: Box<dyn Cryptographic>,

  participant_handle_cache_: HashMap<GuidPrefix, ParticipantCryptoHandle>,
  local_entity_handle_cache_: HashMap<GUID, EntityCryptoHandle>,
  remote_entity_handle_cache_: HashMap<(GUID, GUID), EntityCryptoHandle>,
}

impl SecurityPlugins {
  pub fn new(
    auth: Box<impl Authentication + 'static>,
    access: Box<impl AccessControl + 'static>,
    crypto: Box<impl Cryptographic + 'static>,
  ) -> Self {
    Self {
      auth,
      access,
      crypto,
      participant_handle_cache_: HashMap::new(),
      local_entity_handle_cache_: HashMap::new(),
      remote_entity_handle_cache_: HashMap::new(),
    }
  }

  fn get_participant_handle(
    &self,
    guid_prefix: &GuidPrefix,
  ) -> SecurityResult<ParticipantCryptoHandle> {
    self
      .participant_handle_cache_
      .get(guid_prefix)
      .ok_or(security_error!(
        "Could not find a ParticipantCryptoHandle for the GuidPrefix {:?}",
        guid_prefix
      ))
      .copied()
  }

  fn get_local_entity_handle(&self, guid: &GUID) -> SecurityResult<ParticipantCryptoHandle> {
    self
      .local_entity_handle_cache_
      .get(guid)
      .ok_or(security_error!(
        "Could not find a local EntityHandle for the GUID {:?}",
        guid
      ))
      .copied()
  }

  /// The `local_proxy_guid_pair` should be `&(local_entity_guid, proxy_guid)`.
  fn get_remote_entity_handle(
    &self,
    (local_entity_guid, proxy_guid): (&GUID, &GUID),
  ) -> SecurityResult<ParticipantCryptoHandle> {
    let local_and_proxy_guid_pair = (*local_entity_guid, *proxy_guid);
    self
      .remote_entity_handle_cache_
      .get(&local_and_proxy_guid_pair)
      .ok_or(security_error!(
        "Could not find a remote EntityHandle for the (local_entity_guid, proxy_guid) pair {:?}",
        local_and_proxy_guid_pair
      ))
      .copied()
  }
}

/// Interface for using the CryptoKeyTransform of the Cryptographic plugin
impl SecurityPlugins {
  pub fn encode_datawriter_submessage(
    &self,
    plain_submessage: Submessage,
    source_guid: &GUID,
    destination_guid_list: &[GUID],
  ) -> SecurityResult<EncodedSubmessage> {
    self.crypto.encode_datawriter_submessage(
      plain_submessage,
      self.get_local_entity_handle(source_guid)?,
      SecurityResult::from_iter(
        destination_guid_list
          .iter()
          .map(|destination_guid| self.get_remote_entity_handle((source_guid, destination_guid))),
      )?,
    )
  }

  pub fn encode_datareader_submessage(
    &self,
    plain_submessage: Submessage,
    source_guid: &GUID,
    destination_guid_list: &[GUID],
  ) -> SecurityResult<EncodedSubmessage> {
    self.crypto.encode_datareader_submessage(
      plain_submessage,
      self.get_local_entity_handle(source_guid)?,
      SecurityResult::from_iter(
        destination_guid_list
          .iter()
          .map(|destination_guid| self.get_remote_entity_handle((source_guid, destination_guid))),
      )?,
    )
  }

  pub fn encode_message(
    &self,
    plain_message: Message,
    source_guid_prefix: &GuidPrefix,
    destination_guid_prefix_list: &[GuidPrefix],
  ) -> SecurityResult<Message> {
    self.crypto.encode_rtps_message(
      plain_message,
      self.get_participant_handle(source_guid_prefix)?,
      SecurityResult::from_iter(
        destination_guid_prefix_list
          .iter()
          .map(|destination_guid| self.get_participant_handle(destination_guid)),
      )?,
    )
  }

  pub fn decode_rtps_message(
    &self,
    encoded_message: Message,
    source_guid_prefix: &GuidPrefix,
    destination_guid_prefix: &GuidPrefix,
  ) -> SecurityResult<Message> {
    self.crypto.decode_rtps_message(
      encoded_message,
      self.get_participant_handle(destination_guid_prefix)?,
      self.get_participant_handle(source_guid_prefix)?,
    )
  }

  pub fn preprocess_secure_submessage(
    &self,
    secure_prefix: &SecurePrefix,
    source_guid_prefix: &GuidPrefix,
    destination_guid_prefix: &GuidPrefix,
  ) -> SecurityResult<SecureSubmessageCategory> {
    self.crypto.preprocess_secure_submsg(
      secure_prefix,
      self.get_participant_handle(destination_guid_prefix)?,
      self.get_participant_handle(source_guid_prefix)?,
    )
  }

  pub fn decode_datawriter_submessage(
    &self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    receiving_datareader_crypto: DatareaderCryptoHandle,
    sending_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<WriterSubmessage> {
    self.crypto.decode_datawriter_submessage(
      encoded_rtps_submessage,
      receiving_datareader_crypto,
      sending_datawriter_crypto,
    )
  }

  pub fn decode_datareader_submessage(
    &self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    receiving_datawriter_crypto: DatawriterCryptoHandle,
    sending_datareader_crypto: DatareaderCryptoHandle,
  ) -> SecurityResult<ReaderSubmessage> {
    self.crypto.decode_datareader_submessage(
      encoded_rtps_submessage,
      receiving_datawriter_crypto,
      sending_datareader_crypto,
    )
  }
}

#[derive(Clone)]
pub(crate) struct SecurityPluginsHandle {
  inner: Arc<Mutex<SecurityPlugins>>,
}

impl SecurityPluginsHandle {
  pub(crate) fn new(s: SecurityPlugins) -> Self {
    Self {
      inner: Arc::new(Mutex::new(s)),
    }
  }
}

impl fmt::Debug for SecurityPluginsHandle {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("SecurityPluginsHandle")
  }
}

impl std::ops::Deref for SecurityPluginsHandle {
  type Target = Mutex<SecurityPlugins>;
  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
