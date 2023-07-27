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
    DatareaderCryptoHandle, DatawriterCryptoHandle, EntityCryptoHandle, ParticipantCryptoHandle,
    SecureSubmessageCategory,
  },
  AccessControl, Authentication, Cryptographic, SecurityError, SecurityResult,
};

pub(crate) struct SecurityPlugins {
  pub auth: Box<dyn Authentication>,
  pub access: Box<dyn AccessControl>,
  crypto: Box<dyn Cryptographic>,

  participant_handle_cache: HashMap<GuidPrefix, ParticipantCryptoHandle>,
  entity_handle_cache: HashMap<GUID, EntityCryptoHandle>,
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
      participant_handle_cache: HashMap::new(),
      entity_handle_cache: HashMap::new(),
    }
  }

  fn get_participant_handle(
    &self,
    guid_prefix: &GuidPrefix,
  ) -> SecurityResult<ParticipantCryptoHandle> {
    self
      .participant_handle_cache
      .get(guid_prefix)
      .ok_or(security_error!(
        "Could not find a ParticipantCryptoHandle for the GuidPrefix {:?}",
        guid_prefix
      ))
      .copied()
  }
}

/// Interface for using the CryptoKeyTransform of the Cryptographic plugin
impl SecurityPlugins {
  pub fn decode_rtps_message(
    &self,
    encoded_message: Message,
    source_guid_prefix: &GuidPrefix,
    dest_guid_prefix: &GuidPrefix,
  ) -> SecurityResult<Message> {
    self.crypto.decode_rtps_message(
      encoded_message,
      self.get_participant_handle(dest_guid_prefix)?,
      self.get_participant_handle(source_guid_prefix)?,
    )
  }

  pub fn preprocess_secure_submessage(
    &self,
    secure_prefix: &SecurePrefix,
    source_guid_prefix: &GuidPrefix,
    dest_guid_prefix: &GuidPrefix,
  ) -> SecurityResult<SecureSubmessageCategory> {
    self.crypto.preprocess_secure_submsg(
      secure_prefix,
      self.get_participant_handle(dest_guid_prefix)?,
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
