use core::fmt;
use std::{
  collections::{HashMap, HashSet},
  sync::{Arc, Mutex, MutexGuard},
};

use log::{debug, error};

use crate::{
  discovery::{DiscoveredReaderData, DiscoveredWriterData},
  messages::submessages::{
    elements::parameter_list::ParameterList, secure_postfix::SecurePostfix,
    secure_prefix::SecurePrefix,
  },
  qos,
  rtps::{Message, Submessage, SubmessageBody},
  security_error,
  structure::guid::{EntityId, GuidPrefix},
  QosPolicies, GUID,
};
use super::{
  access_control::*,
  authentication::*,
  cryptographic::{
    DatareaderCryptoHandle, DatareaderCryptoToken, DatawriterCryptoHandle, DatawriterCryptoToken,
    DecodedSubmessage, EncodedSubmessage, EndpointCryptoHandle, ParticipantCryptoHandle,
    ParticipantCryptoToken,
  },
  types::*,
  AccessControl, Cryptographic,
};

pub(crate) struct SecurityPlugins {
  auth: Box<dyn Authentication>,
  access: Box<dyn AccessControl>,
  crypto: Box<dyn Cryptographic>,

  identity_handle_cache: HashMap<GuidPrefix, IdentityHandle>,
  permissions_handle_cache: HashMap<GuidPrefix, PermissionsHandle>,
  handshake_handle_cache: HashMap<GuidPrefix, HandshakeHandle>,

  local_participant_crypto_handle: Option<ParticipantCryptoHandle>,
  remote_participant_crypto_handle_cache: HashMap<GuidPrefix, ParticipantCryptoHandle>,
  local_endpoint_crypto_handle_cache: HashMap<GUID, EndpointCryptoHandle>,
  remote_endpoint_crypto_handle_cache: HashMap<(GUID, GUID), EndpointCryptoHandle>,

  // Guid prefixes or guids of unprotected domains and topics to allow skipping plugin calls when
  // there are no CryptoHeaders
  rtps_not_protected: HashSet<GuidPrefix>,
  submessage_not_protected: HashSet<GUID>,
  payload_not_protected: HashSet<GUID>,

  test_disable_crypto_transform: bool, /* TODO: Disables the crypto transform interface, remove
                                        * after testing */
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
      identity_handle_cache: HashMap::new(),
      permissions_handle_cache: HashMap::new(),
      handshake_handle_cache: HashMap::new(),
      local_participant_crypto_handle: None,
      remote_participant_crypto_handle_cache: HashMap::new(),
      local_endpoint_crypto_handle_cache: HashMap::new(),
      remote_endpoint_crypto_handle_cache: HashMap::new(),

      rtps_not_protected: HashSet::new(),
      submessage_not_protected: HashSet::new(),
      payload_not_protected: HashSet::new(),

      test_disable_crypto_transform: false, // TODO Remove after testing
    }
  }

  fn get_identity_handle(&self, guidp: &GuidPrefix) -> SecurityResult<IdentityHandle> {
    self
      .identity_handle_cache
      .get(guidp)
      .ok_or_else(|| {
        security_error!(
          "Could not find an IdentityHandle for the GUID prefix {:?}",
          guidp
        )
      })
      .copied()
  }

  fn get_permissions_handle(&self, guidp: &GuidPrefix) -> SecurityResult<PermissionsHandle> {
    self
      .permissions_handle_cache
      .get(guidp)
      .ok_or_else(|| {
        security_error!(
          "Could not find a PermissionsHandle for the GUID prefix {:?}",
          guidp
        )
      })
      .copied()
  }

  fn get_handshake_handle(&self, remote_guidp: &GuidPrefix) -> SecurityResult<HandshakeHandle> {
    self
      .handshake_handle_cache
      .get(remote_guidp)
      .ok_or_else(|| {
        security_error!(
          "Could not find a HandshakeHandle for the GUID prefix {:?}",
          remote_guidp
        )
      })
      .copied()
  }

  fn get_local_participant_crypto_handle(&self) -> SecurityResult<ParticipantCryptoHandle> {
    self
      .local_participant_crypto_handle
      .ok_or_else(|| security_error("Local participant crypto handle has not been set"))
  }

  fn get_remote_participant_crypto_handle(
    &self,
    guid_prefix: &GuidPrefix,
  ) -> SecurityResult<ParticipantCryptoHandle> {
    self
      .remote_participant_crypto_handle_cache
      .get(guid_prefix)
      .ok_or_else(|| {
        security_error!(
          "Could not find a ParticipantCryptoHandle for the GuidPrefix {:?}",
          guid_prefix
        )
      })
      .copied()
  }

  fn get_local_endpoint_crypto_handle(&self, guid: &GUID) -> SecurityResult<EndpointCryptoHandle> {
    self
      .local_endpoint_crypto_handle_cache
      .get(guid)
      .ok_or_else(|| {
        security_error!(
          "Could not find a local EndpointCryptoHandle for the GUID {:?}",
          guid
        )
      })
      .copied()
  }

  fn insert_to_identity_handle_cache(
    &mut self,
    participant_guidp: GuidPrefix,
    handle: IdentityHandle,
  ) {
    let old_opt = self.identity_handle_cache.insert(participant_guidp, handle);
    if let Some(old) = old_opt {
      debug!(
        "Replaced IdentityHandle for guid prefix {:?}. Old: {}, new: {}",
        participant_guidp, old, handle
      );
    }
  }

  fn insert_to_permissions_handle_cache(
    &mut self,
    participant_guidp: GuidPrefix,
    handle: PermissionsHandle,
  ) {
    let old_opt = self
      .permissions_handle_cache
      .insert(participant_guidp, handle);
    if let Some(old) = old_opt {
      debug!(
        "Replaced PermissionsHandle for guid prefix {:?}. Old: {}, new: {}",
        participant_guidp, old, handle
      );
    }
  }

  fn insert_to_remote_participant_crypto_handle_cache(
    &mut self,
    participant_guidp: GuidPrefix,
    handle: ParticipantCryptoHandle,
  ) {
    let old_opt = self
      .remote_participant_crypto_handle_cache
      .insert(participant_guidp, handle);
    if let Some(old) = old_opt {
      debug!(
        "Replaced ParticipantCryptoHandle for guid prefix {:?}. Old: {}, new: {}",
        participant_guidp, old, handle
      );
    }
  }

  // Checks whether the given guid matches one of the given handles
  pub fn confirm_local_endpoint_guid(
    &self,
    local_endpoint_crypto_handles: &[EndpointCryptoHandle],
    guid: &GUID,
  ) -> bool {
    self
      .local_endpoint_crypto_handle_cache
      .get(guid)
      .map_or(false, |found_handle| {
        local_endpoint_crypto_handles.contains(found_handle)
      })
  }

  /// The `local_proxy_guid_pair` should be `&(local_endpoint_guid,
  /// proxy_guid)`.
  fn get_remote_endpoint_crypto_handle(
    &self,
    (local_endpoint_guid, proxy_guid): (&GUID, &GUID),
  ) -> SecurityResult<EndpointCryptoHandle> {
    let local_and_proxy_guid_pair = (*local_endpoint_guid, *proxy_guid);
    self
      .remote_endpoint_crypto_handle_cache
      .get(&local_and_proxy_guid_pair)
      .ok_or_else(|| {
        security_error!(
          "Could not find a remote EndpointCryptoHandle for the (local_endpoint_guid, proxy_guid) \
           pair {:?}",
          local_and_proxy_guid_pair
        )
      })
      .copied()
  }

  fn store_remote_endpoint_crypto_handle(
    &mut self,
    (local_endpoint_guid, remote_endpoint_guid): (GUID, GUID),
    handle: EndpointCryptoHandle,
  ) {
    let local_and_remote_guid_pair = (local_endpoint_guid, remote_endpoint_guid);
    let old_opt = self
      .remote_endpoint_crypto_handle_cache
      .insert(local_and_remote_guid_pair, handle);
    if let Some(old) = old_opt {
      debug!(
        "Replaced EndpointCryptoHandle for guid pair {:?}. Old: {}, new: {}",
        local_and_remote_guid_pair, old, handle
      );
    }
  }
}

/// Interface for using the Authentication plugin
impl SecurityPlugins {
  pub fn validate_local_identity(
    &mut self,
    domain_id: u16,
    participant_qos: &QosPolicies,
    candidate_participant_guid: GUID,
  ) -> SecurityResult<GUID> {
    let (outcome, identity_handle, sec_guid) =
      self
        .auth
        .validate_local_identity(domain_id, participant_qos, candidate_participant_guid)?;

    if let ValidationOutcome::Ok = outcome {
      // Everything OK, store handle and return GUID
      self.insert_to_identity_handle_cache(sec_guid.prefix, identity_handle);
      Ok(sec_guid)
    } else {
      // If the builtin authentication does not fail, it should produce only OK
      // outcome. If some other outcome was produced, return an error
      Err(security_error!(
        "Validating local identity produced an unexpected outcome"
      ))
    }
  }

  pub fn get_identity_token(&self, participant_guidp: GuidPrefix) -> SecurityResult<IdentityToken> {
    let identity_handle = self.get_identity_handle(&participant_guidp)?;
    self.auth.get_identity_token(identity_handle)
  }

  pub fn get_identity_status_token(
    &self,
    participant_guidp: GuidPrefix,
  ) -> SecurityResult<IdentityStatusToken> {
    let identity_handle = self.get_identity_handle(&participant_guidp)?;
    self.auth.get_identity_status_token(identity_handle)
  }

  pub fn set_permissions_credential_and_token(
    &mut self,
    participant_guidp: GuidPrefix,
    permissions_credential_token: PermissionsCredentialToken,
    permissions_token: PermissionsToken,
  ) -> SecurityResult<()> {
    let handle = self.get_identity_handle(&participant_guidp)?;
    self.auth.set_permissions_credential_and_token(
      handle,
      permissions_credential_token,
      permissions_token,
    )
  }

  pub fn validate_remote_identity(
    &mut self,
    local_participant_guidp: GuidPrefix,
    remote_identity_token: IdentityToken,
    remote_participant_guidp: GuidPrefix,
    remote_auth_request_token: Option<AuthRequestMessageToken>,
  ) -> SecurityResult<(ValidationOutcome, Option<AuthRequestMessageToken>)> {
    let local_identity_handle = self.get_identity_handle(&local_participant_guidp)?;

    let (outcome, remote_id_handle, auth_req_token_opt) = self.auth.validate_remote_identity(
      remote_auth_request_token,
      local_identity_handle,
      remote_identity_token,
      remote_participant_guidp,
    )?;

    // Add remote identity handle to cache
    self.insert_to_identity_handle_cache(remote_participant_guidp, remote_id_handle);

    Ok((outcome, auth_req_token_opt))
  }

  pub fn begin_handshake_request(
    &mut self,
    local_guidp: GuidPrefix,
    remote_guidp: GuidPrefix,
    serialized_local_participant_data: Vec<u8>,
  ) -> SecurityResult<(ValidationOutcome, HandshakeMessageToken)> {
    let initiator_identity_handle = self.get_identity_handle(&local_guidp)?;
    let replier_identity_handle = self.get_identity_handle(&remote_guidp)?;

    let (outcome, handshake_handle, handshake_token) = self.auth.begin_handshake_request(
      initiator_identity_handle,
      replier_identity_handle,
      serialized_local_participant_data,
    )?;

    // Store handshake handle
    self
      .handshake_handle_cache
      .insert(remote_guidp, handshake_handle);

    Ok((outcome, handshake_token))
  }

  pub fn begin_handshake_reply(
    &mut self,
    local_participant_guidp: GuidPrefix,
    remote_participant_guidp: GuidPrefix,
    handshake_message_in: HandshakeMessageToken,
    serialized_local_participant_data: Vec<u8>,
  ) -> SecurityResult<(ValidationOutcome, HandshakeMessageToken)> {
    let initiator_identity_handle = self.get_identity_handle(&remote_participant_guidp)?;
    let replier_identity_handle = self.get_identity_handle(&local_participant_guidp)?;

    let (outcome, handshake_handle, handshake_token) = self.auth.begin_handshake_reply(
      handshake_message_in,
      initiator_identity_handle,
      replier_identity_handle,
      serialized_local_participant_data,
    )?;

    // Store handshake handle
    self
      .handshake_handle_cache
      .insert(remote_participant_guidp, handshake_handle);

    Ok((outcome, handshake_token))
  }

  pub fn process_handshake(
    &mut self,
    remote_participant_guidp: GuidPrefix,
    handshake_message_in: HandshakeMessageToken,
  ) -> SecurityResult<(ValidationOutcome, Option<HandshakeMessageToken>)> {
    let handshake_handle = self.get_handshake_handle(&remote_participant_guidp)?;

    self
      .auth
      .process_handshake(handshake_message_in, handshake_handle)
  }

  pub fn get_authenticated_peer_credential_token(
    &self,
    remote_participant_guidp: GuidPrefix,
  ) -> SecurityResult<AuthenticatedPeerCredentialToken> {
    let handshake_handle = self.get_handshake_handle(&remote_participant_guidp)?;

    self
      .auth
      .get_authenticated_peer_credential_token(handshake_handle)
  }

  pub fn get_shared_secret(
    &self,
    remote_participant_guidp: GuidPrefix,
  ) -> SecurityResult<SharedSecretHandle> {
    let handle = self.get_identity_handle(&remote_participant_guidp)?;
    self.auth.get_shared_secret(handle)
  }
}

/// Interface for using the Access control plugin
impl SecurityPlugins {
  pub fn validate_local_permissions(
    &mut self,
    domain_id: u16,
    participant_guidp: GuidPrefix,
    participant_qos: &QosPolicies,
  ) -> SecurityResult<()> {
    let identity_handle = self.get_identity_handle(&participant_guidp)?;
    let permissions_handle = self.access.validate_local_permissions(
      &*self.auth,
      identity_handle,
      domain_id,
      participant_qos,
    )?;
    self.insert_to_permissions_handle_cache(participant_guidp, permissions_handle);
    Ok(())
  }

  pub fn validate_remote_permissions(
    &mut self,
    local_participant_guidp: GuidPrefix,
    remote_participant_guidp: GuidPrefix,
    remote_permissions_token: &PermissionsToken,
    remote_credential_token: &AuthenticatedPeerCredentialToken,
  ) -> SecurityResult<()> {
    let local_id_handle = self.get_identity_handle(&local_participant_guidp)?;
    let remote_id_handle = self.get_identity_handle(&remote_participant_guidp)?;

    let permissions_handle = self.access.validate_remote_permissions(
      &*self.auth,
      local_id_handle,
      remote_id_handle,
      remote_permissions_token,
      remote_credential_token,
    )?;

    self.insert_to_permissions_handle_cache(remote_participant_guidp, permissions_handle);
    Ok(())
  }

  pub fn check_create_participant(
    &self,
    domain_id: u16,
    participant_guidp: GuidPrefix,
    qos: &QosPolicies,
  ) -> SecurityResult<bool> {
    let handle = self.get_permissions_handle(&participant_guidp)?;
    self.access.check_create_participant(handle, domain_id, qos)
  }

  pub fn check_remote_participant(
    &self,
    domain_id: u16,
    participant_guidp: GuidPrefix,
  ) -> SecurityResult<bool> {
    let handle = self.get_permissions_handle(&participant_guidp)?;
    self
      .access
      .check_remote_participant(handle, domain_id, None)
  }

  // This function is called when DataReaders from non-secure discovery
  // need to be checked
  pub fn check_remote_datareader_from_nonsecure(
    &self,
    participant_guidp: GuidPrefix,
    domain_id: u16,
    reader_data: &DiscoveredReaderData,
  ) -> SecurityResult<(bool, bool)> {
    let handle = self.get_permissions_handle(&participant_guidp)?;
    // Convert normal DiscoveredReaderData to SubscriptionBuiltinTopicDataSecure,
    // which is what Access control plugin expects
    let secure_sub_data = SubscriptionBuiltinTopicDataSecure::from(reader_data.clone());
    self
      .access
      .check_remote_datareader(handle, domain_id, &secure_sub_data)
  }

  // This function is called when DataWriters from non-secure discovery
  // need to be checked
  pub fn check_remote_datawriter_from_nonsecure(
    &self,
    participant_guidp: GuidPrefix,
    domain_id: u16,
    writer_data: &DiscoveredWriterData,
  ) -> SecurityResult<bool> {
    let handle = self.get_permissions_handle(&participant_guidp)?;
    // Convert normal DiscoveredWriterData to PublicationBuiltinTopicDataSecure,
    // which is what Access control plugin expects
    let secure_pub_data = PublicationBuiltinTopicDataSecure::from(writer_data.clone());
    self
      .access
      .check_remote_datawriter(handle, domain_id, &secure_pub_data)
  }

  pub fn get_permissions_token(
    &self,
    participant_guidp: GuidPrefix,
  ) -> SecurityResult<PermissionsToken> {
    let handle: PermissionsHandle = self.get_permissions_handle(&participant_guidp)?;
    self.access.get_permissions_token(handle)
  }

  pub fn get_permissions_credential_token(
    &self,
    participant_guidp: GuidPrefix,
  ) -> SecurityResult<PermissionsCredentialToken> {
    let handle: PermissionsHandle = self.get_permissions_handle(&participant_guidp)?;
    self.access.get_permissions_credential_token(handle)
  }

  pub fn get_participant_sec_attributes(
    &self,
    participant_guidp: GuidPrefix,
  ) -> SecurityResult<ParticipantSecurityAttributes> {
    let handle: PermissionsHandle = self.get_permissions_handle(&participant_guidp)?;
    self.access.get_participant_sec_attributes(handle)
  }

  pub fn get_topic_sec_attributes(
    &self,
    participant_guidp: GuidPrefix,
    topic_name: &str,
  ) -> SecurityResult<TopicSecurityAttributes> {
    let permissions_handle = self.get_permissions_handle(&participant_guidp)?;
    self
      .access
      .get_topic_sec_attributes(permissions_handle, topic_name)
  }

  pub fn get_reader_sec_attributes(
    &self,
    reader_guid: GUID,
    topic_name: String,
  ) -> SecurityResult<EndpointSecurityAttributes> {
    let handle = self.get_permissions_handle(&reader_guid.prefix)?;
    self
      .access
      .get_datareader_sec_attributes(handle, topic_name)
  }

  pub fn get_writer_sec_attributes(
    &self,
    writer_guid: GUID,
    topic_name: String,
  ) -> SecurityResult<EndpointSecurityAttributes> {
    let handle = self.get_permissions_handle(&writer_guid.prefix)?;
    self
      .access
      .get_datawriter_sec_attributes(handle, topic_name)
  }
}

/// Interface for using the CryptoKeyFactory of the Cryptographic plugin
impl SecurityPlugins {
  pub fn register_local_participant(
    &mut self,
    participant_guidp: GuidPrefix,
    participant_properties: Option<qos::policy::Property>,
    participant_security_attributes: ParticipantSecurityAttributes,
  ) -> SecurityResult<()> {
    let identity_handle = self.get_identity_handle(&participant_guidp)?;
    let permissions_handle = self.get_permissions_handle(&participant_guidp)?;

    let properties = participant_properties
      .map(|prop| prop.value)
      .unwrap_or_default();

    if !participant_security_attributes.is_rtps_protected {
      self.rtps_not_protected.insert(participant_guidp);
    }

    let crypto_handle = self.crypto.register_local_participant(
      identity_handle,
      permissions_handle,
      &properties,
      participant_security_attributes,
    )?;

    self.local_participant_crypto_handle = Some(crypto_handle);
    Ok(())
  }

  pub fn register_local_reader(
    &mut self,
    reader_guid: GUID,
    reader_properties: Option<qos::policy::Property>,
    reader_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<()> {
    let local_participant_crypto_handle = self.get_local_participant_crypto_handle()?;

    let mut properties = reader_properties.map(|prop| prop.value).unwrap_or_default();

    if reader_guid.entity_id == EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_READER {
      // Add a property which the crypto plugin expects for the Volatile Reader
      // (see 8.8.8.1 of the Security spec)
      properties.push(volatile_reader_recognition_property());
    }

    if !reader_security_attributes.is_submessage_protected {
      self.submessage_not_protected.insert(reader_guid);
    }
    if !reader_security_attributes.is_payload_protected {
      self.payload_not_protected.insert(reader_guid);
    }

    let crypto_handle = self.crypto.register_local_datareader(
      local_participant_crypto_handle,
      &properties,
      reader_security_attributes,
    )?;

    self
      .local_endpoint_crypto_handle_cache
      .insert(reader_guid, crypto_handle);
    Ok(())
  }

  pub fn register_local_writer(
    &mut self,
    writer_guid: GUID,
    writer_properties: Option<qos::policy::Property>,
    writer_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<()> {
    let local_participant_crypto_handle = self.get_local_participant_crypto_handle()?;

    let mut properties = writer_properties.map(|prop| prop.value).unwrap_or_default();

    if writer_guid.entity_id == EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_WRITER {
      // Add a property which the crypto plugin expects for the Volatile Writer
      // (see 8.8.8.1 of the Security spec)
      properties.push(volatile_writer_recognition_property());
    }

    if !writer_security_attributes.is_submessage_protected {
      self.submessage_not_protected.insert(writer_guid);
    }
    if !writer_security_attributes.is_payload_protected {
      self.payload_not_protected.insert(writer_guid);
    }

    let crypto_handle = self.crypto.register_local_datawriter(
      local_participant_crypto_handle,
      &properties,
      writer_security_attributes,
    )?;

    self
      .local_endpoint_crypto_handle_cache
      .insert(writer_guid, crypto_handle);
    Ok(())
  }

  pub fn register_matched_remote_participant(
    &mut self,
    remote_participant_guidp: GuidPrefix,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<()> {
    let local_crypto = self.get_local_participant_crypto_handle()?;
    let remote_identity = self.get_identity_handle(&remote_participant_guidp)?;
    let remote_permissions = self.get_permissions_handle(&remote_participant_guidp)?;

    let remote_crypto_handle = self.crypto.register_matched_remote_participant(
      local_crypto,
      remote_identity,
      remote_permissions,
      shared_secret,
    )?;

    self.insert_to_remote_participant_crypto_handle_cache(
      remote_participant_guidp,
      remote_crypto_handle,
    );
    Ok(())
  }

  pub fn register_matched_remote_reader(
    &mut self,
    remote_reader_guid: GUID,
    local_writer_guid: GUID,
    relay_only: bool,
  ) -> SecurityResult<()> {
    // First get the secret shared by the participants
    let shared_secret = self.get_shared_secret(remote_reader_guid.prefix)?;

    let local_writer_crypto = self.get_local_endpoint_crypto_handle(&local_writer_guid)?;
    let remote_participant_crypto =
      self.get_remote_participant_crypto_handle(&remote_reader_guid.prefix)?;

    let remote_reader_crypto = self.crypto.register_matched_remote_datareader(
      local_writer_crypto,
      remote_participant_crypto,
      shared_secret,
      relay_only,
    )?;

    self.store_remote_endpoint_crypto_handle(
      (local_writer_guid, remote_reader_guid),
      remote_reader_crypto,
    );
    Ok(())
  }

  pub fn register_matched_remote_writer(
    &mut self,
    remote_writer_guid: GUID,
    local_reader_guid: GUID,
  ) -> SecurityResult<()> {
    // First get the secret shared by the participants
    let shared_secret = self.get_shared_secret(remote_writer_guid.prefix)?;

    let local_reader_crypto = self.get_local_endpoint_crypto_handle(&local_reader_guid)?;
    let remote_participant_crypto =
      self.get_remote_participant_crypto_handle(&remote_writer_guid.prefix)?;

    let remote_writer_crypto = self.crypto.register_matched_remote_datawriter(
      local_reader_crypto,
      remote_participant_crypto,
      shared_secret,
    )?;

    self.store_remote_endpoint_crypto_handle(
      (local_reader_guid, remote_writer_guid),
      remote_writer_crypto,
    );
    Ok(())
  }

  pub fn unregister_local_reader(&mut self, reader_guid: &GUID) -> SecurityResult<()> {
    let handle = self.get_local_endpoint_crypto_handle(reader_guid)?;
    self.crypto.unregister_datareader(handle)
  }

  pub fn unregister_local_writer(&mut self, writer_guid: &GUID) -> SecurityResult<()> {
    let handle = self.get_local_endpoint_crypto_handle(writer_guid)?;
    self.crypto.unregister_datawriter(handle)
  }
}

/// Interface for using the CryptoKeyExchange of the Cryptographic plugin
impl SecurityPlugins {
  pub fn create_local_participant_crypto_tokens(
    &mut self,
    remote_participant_guidp: GuidPrefix,
  ) -> SecurityResult<Vec<ParticipantCryptoToken>> {
    let local_crypto_handle = self.get_local_participant_crypto_handle()?;
    let remote_crypto_handle =
      self.get_remote_participant_crypto_handle(&remote_participant_guidp)?;

    self
      .crypto
      .create_local_participant_crypto_tokens(local_crypto_handle, remote_crypto_handle)
  }

  pub fn create_local_writer_crypto_tokens(
    &mut self,
    local_writer_guid: GUID,
    remote_reader_guid: GUID,
  ) -> SecurityResult<Vec<ParticipantCryptoToken>> {
    let local_writer_crypto_handle = self.get_local_endpoint_crypto_handle(&local_writer_guid)?;
    let remote_reader_crypto_handle =
      self.get_remote_endpoint_crypto_handle((&local_writer_guid, &remote_reader_guid))?;

    self.crypto.create_local_datawriter_crypto_tokens(
      local_writer_crypto_handle,
      remote_reader_crypto_handle,
    )
  }

  pub fn create_local_reader_crypto_tokens(
    &mut self,
    local_reader_guid: GUID,
    remote_writer_guid: GUID,
  ) -> SecurityResult<Vec<ParticipantCryptoToken>> {
    let local_reader_crypto_handle = self.get_local_endpoint_crypto_handle(&local_reader_guid)?;
    let remote_writer_crypto_handle =
      self.get_remote_endpoint_crypto_handle((&local_reader_guid, &remote_writer_guid))?;

    self.crypto.create_local_datareader_crypto_tokens(
      local_reader_crypto_handle,
      remote_writer_crypto_handle,
    )
  }

  pub fn set_remote_participant_crypto_tokens(
    &mut self,
    remote_participant_guidp: GuidPrefix,
    remote_participant_tokens: Vec<ParticipantCryptoToken>,
  ) -> SecurityResult<()> {
    let local_crypto_handle = self.get_local_participant_crypto_handle()?;
    let remote_crypto_handle =
      self.get_remote_participant_crypto_handle(&remote_participant_guidp)?;

    self.crypto.set_remote_participant_crypto_tokens(
      local_crypto_handle,
      remote_crypto_handle,
      remote_participant_tokens,
    )
  }

  pub fn set_remote_writer_crypto_tokens(
    &mut self,
    remote_writer_guid: GUID,
    local_reader_guid: GUID,
    remote_crypto_tokens: Vec<DatawriterCryptoToken>,
  ) -> SecurityResult<()> {
    let local_reader_crypto_handle = self.get_local_endpoint_crypto_handle(&local_reader_guid)?;
    let remote_writer_crypto_handle =
      self.get_remote_endpoint_crypto_handle((&local_reader_guid, &remote_writer_guid))?;

    self.crypto.set_remote_datawriter_crypto_tokens(
      local_reader_crypto_handle,
      remote_writer_crypto_handle,
      remote_crypto_tokens,
    )
  }

  pub fn set_remote_reader_crypto_tokens(
    &mut self,
    remote_reader_guid: GUID,
    local_writer_guid: GUID,
    remote_crypto_tokens: Vec<DatareaderCryptoToken>,
  ) -> SecurityResult<()> {
    let local_writer_crypto_handle = self.get_local_endpoint_crypto_handle(&local_writer_guid)?;
    let remote_reader_crypto_handle =
      self.get_remote_endpoint_crypto_handle((&local_writer_guid, &remote_reader_guid))?;

    self.crypto.set_remote_datareader_crypto_tokens(
      local_writer_crypto_handle,
      remote_reader_crypto_handle,
      remote_crypto_tokens,
    )
  }
}

/// Interface for using the CryptoTransform of the Cryptographic plugin
impl SecurityPlugins {
  /// Calls [super::cryptographic::cryptographic_plugin::CryptoTransform::encode_serialized_payload]
  pub fn encode_serialized_payload(
    &self,
    serialized_payload: Vec<u8>,
    sending_datawriter_guid: &GUID,
  ) -> SecurityResult<(Vec<u8>, ParameterList)> {
    // TODO remove after testing, skips encoding
    if self.test_disable_crypto_transform {
      return Ok((serialized_payload, ParameterList::new()));
    }

    if self.payload_not_protected(sending_datawriter_guid) {
      return Ok((serialized_payload, ParameterList::new()));
    }

    self.crypto.encode_serialized_payload(
      serialized_payload,
      self.get_local_endpoint_crypto_handle(sending_datawriter_guid)?,
    )
  }

  /// Calls [super::cryptographic::cryptographic_plugin::CryptoTransform::encode_datawriter_submessage].
  pub fn encode_datawriter_submessage(
    &self,
    plain_submessage: Submessage,
    source_guid: &GUID,
    destination_guid_list: &[GUID],
  ) -> SecurityResult<EncodedSubmessage> {
    // Enforce that the submessage is a writer submessage.
    match plain_submessage.body {
      SubmessageBody::Writer(_) => {
        // TODO remove after testing, skips encoding
        if self.test_disable_crypto_transform {
          return Ok(EncodedSubmessage::Unencoded(plain_submessage));
        }

        if self.submessage_not_protected(source_guid) {
          return Ok(EncodedSubmessage::Unencoded(plain_submessage));
        }

        // Convert the destination GUIDs to crypto handles
        let mut receiving_datareader_crypto_list: Vec<DatareaderCryptoHandle> =
          SecurityResult::from_iter(destination_guid_list.iter().map(|destination_guid| {
            self.get_remote_endpoint_crypto_handle((source_guid, destination_guid))
          }))?;
        // Remove duplicates
        receiving_datareader_crypto_list.sort();
        receiving_datareader_crypto_list.dedup();

        self.crypto.encode_datawriter_submessage(
          plain_submessage,
          self.get_local_endpoint_crypto_handle(source_guid)?,
          receiving_datareader_crypto_list,
        )
      }
      SubmessageBody::Interpreter(_) => Ok(EncodedSubmessage::Unencoded(plain_submessage)),
      SubmessageBody::Reader(_) => Err(security_error!(
        "encode_datawriter_submessage called for a reader submessage"
      )),
      SubmessageBody::Security(_) => Err(security_error!(
        "encode_datawriter_submessage called for a security submessage"
      )),
    }
  }

  /// Calls [super::cryptographic::cryptographic_plugin::CryptoTransform::encode_datareader_submessage]. The body of the submessage shall be [SubmessageBody::Reader].
  pub fn encode_datareader_submessage(
    &self,
    plain_submessage: Submessage,
    source_guid: &GUID,
    destination_guid_list: &[GUID],
  ) -> SecurityResult<EncodedSubmessage> {
    // Enforce that the submessage is a reader submessage.
    match plain_submessage.body {
      SubmessageBody::Reader(_) => {
        // TODO remove after testing, skips encoding
        if self.test_disable_crypto_transform {
          return Ok(EncodedSubmessage::Unencoded(plain_submessage));
        }

        if self.submessage_not_protected(source_guid) {
          return Ok(EncodedSubmessage::Unencoded(plain_submessage));
        }

        // Convert the destination GUIDs to crypto handles
        let mut receiving_datawriter_crypto_list: Vec<DatawriterCryptoHandle> =
          SecurityResult::from_iter(destination_guid_list.iter().map(|destination_guid| {
            self.get_remote_endpoint_crypto_handle((source_guid, destination_guid))
          }))?;
        // Remove duplicates
        receiving_datawriter_crypto_list.sort();
        receiving_datawriter_crypto_list.dedup();

        self.crypto.encode_datareader_submessage(
          plain_submessage,
          self.get_local_endpoint_crypto_handle(source_guid)?,
          receiving_datawriter_crypto_list,
        )
      }
      SubmessageBody::Interpreter(_) => Ok(EncodedSubmessage::Unencoded(plain_submessage)),
      SubmessageBody::Writer(_) => Err(security_error!(
        "encode_datareader_submessage called for a writer submessage"
      )),
      SubmessageBody::Security(_) => Err(security_error!(
        "encode_datareader_submessage called for a security submessage"
      )),
    }
  }

  /// Calls [super::cryptographic::cryptographic_plugin::CryptoTransform::encode_rtps_message]
  // Currently only those RTPS messages whose source is the local participant
  // can be encoded.
  // TODO: add support for other source participants as well?
  pub fn encode_message(
    &self,
    plain_message: Message,
    source_guid_prefix: &GuidPrefix,
    destination_guid_prefix_list: &[GuidPrefix],
  ) -> SecurityResult<Message> {
    // TODO remove after testing, skips encoding
    if self.test_disable_crypto_transform {
      return Ok(plain_message);
    }

    if self.rtps_not_protected(source_guid_prefix) {
      return Ok(plain_message);
    }

    // Convert the destination GUID prefixes to crypto handles
    let mut receiving_datawriter_crypto_list: Vec<DatawriterCryptoHandle> =
      SecurityResult::from_iter(destination_guid_prefix_list.iter().map(
        |destination_guid_prefix| {
          self.get_remote_participant_crypto_handle(destination_guid_prefix)
        },
      ))?;
    // Remove duplicates
    receiving_datawriter_crypto_list.sort();
    receiving_datawriter_crypto_list.dedup();

    self.crypto.encode_rtps_message(
      plain_message,
      self.get_local_participant_crypto_handle()?,
      receiving_datawriter_crypto_list,
    )
  }

  // Currently only those RTPS messages whose destination is the local participant
  // can be decoded.
  // TODO: add support for other destinations as well?
  pub fn decode_rtps_message(
    &self,
    encoded_message: Message,
    source_guid_prefix: &GuidPrefix,
  ) -> SecurityResult<Message> {
    self.crypto.decode_rtps_message(
      encoded_message,
      self.get_local_participant_crypto_handle()?,
      self.get_remote_participant_crypto_handle(source_guid_prefix)?,
    )
  }

  // Currently only those submessages whose destination is the local participant
  // can be preprocessed. TODO: add support for other destinations as well?

  pub fn decode_submessage(
    &self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    source_guid_prefix: &GuidPrefix,
  ) -> SecurityResult<DecodedSubmessage> {
    self.crypto.decode_submessage(
      encoded_rtps_submessage,
      self.get_local_participant_crypto_handle()?,
      self.get_remote_participant_crypto_handle(source_guid_prefix)?,
    )
  }

  pub fn decode_serialized_payload(
    &self,
    encoded_payload: Vec<u8>,
    inline_qos: ParameterList,
    source_guid: &GUID,
    destination_guid: &GUID,
  ) -> SecurityResult<Vec<u8>> {
    // TODO remove after testing, skips decoding
    if self.test_disable_crypto_transform {
      return Ok(encoded_payload);
    }

    if self.payload_not_protected(destination_guid) {
      Ok(encoded_payload)
    } else {
      self.crypto.decode_serialized_payload(
        encoded_payload,
        inline_qos,
        self.get_local_endpoint_crypto_handle(destination_guid)?,
        self.get_remote_endpoint_crypto_handle((destination_guid, source_guid))?,
      )
    }
  }

  // These allow us to accept unprotected messages without CryptoHeaders in
  // domains or topics where they are allowed
  pub fn rtps_not_protected(&self, local_participant_guid_prefix: &GuidPrefix) -> bool {
    self
      .rtps_not_protected
      .contains(local_participant_guid_prefix)
  }
  pub fn submessage_not_protected(&self, local_endpoint_guid: &GUID) -> bool {
    self.submessage_not_protected.contains(local_endpoint_guid)
  }
  pub fn payload_not_protected(&self, local_endpoint_guid: &GUID) -> bool {
    self.payload_not_protected.contains(local_endpoint_guid)
  }
}

#[derive(Clone)]
pub(crate) struct SecurityPluginsHandle {
  inner: Arc<Mutex<SecurityPlugins>>,
  who_has_it: Arc<Mutex<Option<String>>>,
}

impl SecurityPluginsHandle {
  pub(crate) fn new(s: SecurityPlugins) -> Self {
    Self {
      inner: Arc::new(Mutex::new(s)),
      who_has_it: Arc::new(Mutex::new(None)),
    }
  }

  pub(crate) fn get_plugins(&self) -> MutexGuard<SecurityPlugins> {
    let mut count = 0;
    loop {
      match self.try_lock() {
        Ok(guard) => {
          *self.who_has_it.lock().unwrap() = std::thread::current().name().map(|s| s.to_owned());
          return guard;
        }
        Err(std::sync::TryLockError::WouldBlock) => {
          if count > 10 {
            error!(
              "I need my lock!! {:?} Looks like {:?} has it.",
              count,
              self.who_has_it.lock().unwrap()
            );
          }
          count += 1;
          std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Err(poisoned) => {
          security_error!("Security plugins are poisoned! {poisoned:?}");
          panic!("Security plugins are poisoned!");
        }
      }
    } // loop
  } // fn
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
