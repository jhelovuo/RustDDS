use std::{
  collections::HashMap,
  sync::{Arc, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use mio_extras::channel as mio_channel;

use crate::{
  dds::{
    no_key,
    participant::DomainParticipantWeak,
    with_key::{DataSample, Sample},
  },
  network::constant::DiscoveryNotificationType,
  qos, rpc,
  security::{
    access_control::{ParticipantSecurityAttributes, PermissionsToken},
    authentication::{HandshakeMessageToken, IdentityToken, ValidationOutcome},
    security_plugins::{SecurityPlugins, SecurityPluginsHandle},
    ParticipantGenericMessage, ParticipantSecurityInfo, ParticipantStatelessMessage, SecurityError,
    SecurityResult,
  },
  security_error, security_log,
  serialization::pl_cdr_adapters::PlCdrSerialize,
  structure::{entity::RTPSEntity, guid::EntityId},
  RepresentationIdentifier, SequenceNumber, GUID,
};
use super::{discovery_db::DiscoveryDB, Participant_GUID, SpdpDiscoveredParticipantData};

// Enum for authentication status of a remote participant
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum AuthenticationStatus {
  Authenticated,
  Authenticating, // In the process of being authenticated
  Unauthenticated, /* Not authenticated, but still allowed to communicate with in a limited way
                   * (see Security spec section 8.8.2.1) */
  Rejected, // Could not authenticate & should not communicate to
}

struct UnansweredAuthenticationMessage {
  message: ParticipantStatelessMessage,
  remaining_resend_times: u8,
}

impl UnansweredAuthenticationMessage {
  pub fn new(message: ParticipantStatelessMessage) -> Self {
    Self {
      message,
      remaining_resend_times: 10, // Message is resent at most 10 times
    }
  }
}

// This struct is an appendix to Discovery that handles Security-related
// functionality. The intention is that Discovery calls the methods of this
// struct when Security matters needs to be handled.
// SecureDiscovery also stores items which Discovery needs to do security.
// Some local tokens etc. which do not change during runtime are stored here so
// they don't have to be fetched from security plugins every time when needed
pub(crate) struct SecureDiscovery {
  pub security_plugins: SecurityPluginsHandle,
  pub local_participant_guid: GUID,
  pub local_dp_identity_token: IdentityToken,
  pub local_dp_permissions_token: PermissionsToken,
  pub local_dp_property_qos: qos::policy::Property,
  pub local_dp_sec_attributes: ParticipantSecurityAttributes,

  stateless_message_helper: ParticipantStatelessMessageHelper,
  unanswered_authentication_messages: HashMap<GUID, UnansweredAuthenticationMessage>,
}

impl SecureDiscovery {
  pub fn new(
    domain_participant: &DomainParticipantWeak,
    security_plugins: SecurityPluginsHandle,
  ) -> Result<Self, &'static str> {
    // Run the Discovery-related initialization steps of DDS Security spec v1.1
    // Section "8.8.1 Authentication and AccessControl behavior with local
    // DomainParticipant"

    let plugins = security_plugins.lock().unwrap();

    let participant_guid = domain_participant.guid();

    let property_qos = domain_participant
      .qos()
      .property()
      .expect("No property QoS defined even though security is enabled");

    let identity_token = match plugins.get_identity_token(participant_guid) {
      Ok(token) => token,
      Err(_e) => {
        return Err("Could not get IdentityToken");
      }
    };

    let _identity_status_token = match plugins.get_identity_status_token(participant_guid) {
      Ok(token) => token,
      Err(_e) => {
        return Err("Could not get IdentityStatusToken");
      }
    };

    let permissions_token = match plugins.get_permissions_token(participant_guid) {
      Ok(token) => token,
      Err(_e) => {
        return Err("Could not get PermissionsToken");
      }
    };

    let credential_token = match plugins.get_permissions_credential_token(participant_guid) {
      Ok(token) => token,
      Err(_e) => {
        return Err("Could not get PermissionsCredentialToken");
      }
    };

    if plugins
      .set_permissions_credential_and_token(
        participant_guid,
        credential_token,
        permissions_token.clone(),
      )
      .is_err()
    {
      return Err("Could not set permission tokens.");
    }

    let security_attributes = match plugins.get_participant_sec_attributes(participant_guid) {
      Ok(val) => val,
      Err(_e) => {
        return Err("Could not get ParticipantSecurityAttributes");
      }
    };

    drop(plugins); // Drop mutex guard on plugins so that plugins can be moved to self

    Ok(Self {
      security_plugins,
      local_participant_guid: participant_guid,
      local_dp_identity_token: identity_token,
      local_dp_permissions_token: permissions_token,
      local_dp_property_qos: property_qos,
      local_dp_sec_attributes: security_attributes,
      stateless_message_helper: ParticipantStatelessMessageHelper::new(),
      unanswered_authentication_messages: HashMap::new(),
    })
  }

  // Inspect a new sample from the standard DCPSParticipant Builtin Topic
  // Possibly start the authentication protocol
  // Return boolean indicating if normal Discovery can process the sample as usual
  pub fn participant_read(
    &mut self,
    ds: &DataSample<SpdpDiscoveredParticipantData>,
    discovery_db: &Arc<RwLock<DiscoveryDB>>,
    discovery_updated_sender: &mio_channel::SyncSender<DiscoveryNotificationType>,
    auth_msg_writer: &no_key::DataWriter<ParticipantStatelessMessage>,
  ) -> bool {
    match &ds.value {
      Sample::Value(participant_data) => self.participant_data_read(
        participant_data,
        discovery_db,
        discovery_updated_sender,
        auth_msg_writer,
      ),
      Sample::Dispose(participant_guid) => {
        self.participant_dispose_read(participant_guid, discovery_db)
      }
    }
  }

  // This function inspects a data message from normal DCPSParticipant topic
  // The authentication protocol is possibly started
  // The returned boolean tells if normal Discovery is allowed to process
  // the message.
  #[allow(clippy::needless_bool)] // for return value clarity
  fn participant_data_read(
    &mut self,
    participant_data: &SpdpDiscoveredParticipantData,
    discovery_db: &Arc<RwLock<DiscoveryDB>>,
    discovery_updated_sender: &mio_channel::SyncSender<DiscoveryNotificationType>,
    auth_msg_writer: &no_key::DataWriter<ParticipantStatelessMessage>,
  ) -> bool {
    let guid_prefix = participant_data.participant_guid.prefix;

    // Our action depends on the current authentication status of the remote
    let auth_status_opt = discovery_db_read(discovery_db).get_authentication_status(guid_prefix);

    // Here we get an updated authentication status
    let updated_auth_status = match auth_status_opt {
      None => {
        // No prior info on this participant. Check compatibility
        let compatible = self.check_compatibility_with_remote_participant(participant_data);
        if compatible {
          // We're compatible. Try to authenticate with this participant
          // This returns a new authentication status
          self.start_authentication_with_remote(
            participant_data,
            discovery_db,
            discovery_updated_sender,
            auth_msg_writer,
          )
        } else {
          // We're not compatible Security-wise
          if self
            .local_dp_sec_attributes
            .allow_unauthenticated_participants
          {
            // But configuration still allows matching with the participant (in a limited
            // way)
            security_log!(
              "Remote participant has incompatible Security, but matching with it anyways, since \
               configuration allows this. Remote guid: {:?}",
              participant_data.participant_guid
            );
            AuthenticationStatus::Unauthenticated
          } else {
            // Not allowed to match
            security_log!(
              "Remote participant has incompatible Security, not matching with it. Remote guid: \
               {:?}",
              participant_data.participant_guid
            );
            AuthenticationStatus::Rejected
          }
        }
      }
      Some(auth_status) => {
        // This participant already has an authentication status. Keep it unchanged.
        // Just do some debug logging
        if auth_status == AuthenticationStatus::Authenticated {
          debug!(
            "Received a non-secure DCPSParticipant message from an authenticated remote \
             participant (supposedly). Ignoring the message. Remote guid prefix: {:?}",
            guid_prefix
          );
        }
        auth_status
      }
    };

    // Update authentication status to DB
    discovery_db_write(discovery_db).update_authentication_status(guid_prefix, updated_auth_status);

    // Decide if normal Discovery can process the participant message
    if updated_auth_status == AuthenticationStatus::Unauthenticated {
      true
    } else {
      false
    }
  }

  // This function inspects a dispose message from normal DCPSParticipant topic
  // and decides whether to allow Discovery to process the message
  fn participant_dispose_read(
    &self,
    participant_guid: &Participant_GUID,
    discovery_db: &Arc<RwLock<DiscoveryDB>>,
  ) -> bool {
    let guid_prefix = participant_guid.0.prefix;

    let db = discovery_db_read(discovery_db);

    // Permission to process the message depends on the participant's authentication
    // status
    match db.get_authentication_status(guid_prefix) {
      None => {
        // No prior info on this participant. Let the dispose message be processed
        true
      }
      Some(AuthenticationStatus::Unauthenticated) => {
        // Participant has been marked as Unauthenticated. Allow to process.
        true
      }
      Some(other_status) => {
        debug!(
          "Received a dispose message from participant with authentication status: {:?}. \
           Ignoring. Participant guid prefix: {:?}",
          other_status, guid_prefix
        );
        // Do not allow with any other status
        false
      }
    }
  }

  // Return boolean indicating if we're compatible with the remote participant
  fn check_compatibility_with_remote_participant(
    &self,
    remote_data: &SpdpDiscoveredParticipantData,
  ) -> bool {
    // 1. Check identity tokens
    if let Some(token) = remote_data.identity_token.as_ref() {
      // Class ID of identity tokens needs to be the same (Means they implement the
      // same authentication plugin)
      let my_class_id = self.local_dp_identity_token.class_id();
      let remote_class_id = token.class_id();

      if my_class_id != remote_class_id {
        info!(
          "Participants not compatible because of different IdentityToken class IDs. Local \
           id:{my_class_id}, remote id: {remote_class_id}"
        );
        return false;
      }
    } else {
      // Remote participant does not have identity token.
      info!("Participants not compatible because remote does not have IdentityToken");
      return false;
    }

    // 2. Check permission tokens
    if let Some(token) = remote_data.permissions_token.as_ref() {
      // Class ID of permission tokens needs to be the same (Means they implement the
      // same access control plugin)
      let my_class_id = self.local_dp_permissions_token.class_id();
      let remote_class_id = token.class_id();

      if my_class_id != remote_class_id {
        info!(
          "Participants not compatible because of different PermissionsToken class IDs. Local \
           id:{my_class_id}, remote id: {remote_class_id}"
        );
        return false;
      }
    } else {
      // Remote participant does not have a permissions token.
      info!("Participants not compatible because remote does not have PermissionsToken");
      return false;
    }

    // 3. Check security info (see Security specification section 7.2.7)
    if let Some(remote_sec_info) = remote_data.security_info.as_ref() {
      let my_sec_info = ParticipantSecurityInfo::from(self.local_dp_sec_attributes.clone());

      let my_mask = my_sec_info.participant_security_attributes;
      let remote_mask = remote_sec_info.participant_security_attributes;

      let my_plugin_mask = my_sec_info.plugin_participant_security_attributes;
      let remote_plugin_mask = remote_sec_info.plugin_participant_security_attributes;

      // From the spec:
      // "A compatible configuration is defined as having the same value for
      // all of the attributes in the ParticipantSecurityInfo".
      if my_mask.is_valid()
        && remote_mask.is_valid()
        && my_plugin_mask.is_valid()
        && remote_plugin_mask.is_valid()
      {
        // Check equality of security infos when all masks are valid
        if my_sec_info != *remote_sec_info {
          info!("Participants not compatible because of unequal ParticipantSecurityInfos");
          return false;
        }
      } else {
        // But also from the spec:
        // "If the is_valid is set to zero on either of the masks, the comparison
        // between the local and remote setting for the ParticipantSecurityInfo
        // shall ignore the attribute"

        // TODO: Does it actually make sense to ignore the masks if they're not valid?
        // Seems a bit strange. Currently we require that all masks are valid
        info!(
          "Participants not compatible because some ParticipantSecurityInfo masks are not valid"
        );
        return false;
      }
    } else {
      // Remote participant does not have security info.
      info!("Participants not compatible because remote does not have ParticipantSecurityInfo");
      return false;
    }

    // All checks passed: we are compatible
    true
  }

  // This function is called once we have discovered a new remote participant that
  // we're compatible with Security-wise.
  // It contains the first authentication steps described in section 8.8.2
  // "Authentication behavior with discovered DomainParticipant" of the Security
  // specification.
  // The function returns the resulting authentication status of the remote
  fn start_authentication_with_remote(
    &mut self,
    participant_data: &SpdpDiscoveredParticipantData,
    discovery_db: &Arc<RwLock<DiscoveryDB>>,
    discovery_updated_sender: &mio_channel::SyncSender<DiscoveryNotificationType>,
    auth_msg_writer: &no_key::DataWriter<ParticipantStatelessMessage>,
  ) -> AuthenticationStatus {
    // First gather some needed items
    let my_guid = self.local_participant_guid;
    let remote_guid = participant_data.participant_guid;
    let remote_identity_token = participant_data
      .identity_token
      .clone()
      .expect("IdentityToken disappeared"); // Identity token is here since compatibility test passed

    let outcome: ValidationOutcome = match get_security_plugins(&self.security_plugins)
      .validate_remote_identity(my_guid, remote_identity_token, remote_guid, None)
    {
      Ok(res) => {
        // Validation passed. Getting only the validation outcome, ignoring
        // authentication request token which is not used
        res.0
      }
      Err(e) => {
        // Validation failed
        security_log!(
          "Failed to validate the identity of a remote participant with guid: {:?}. Info: {}",
          remote_guid,
          e.msg
        );
        // See if we can treat the participant as Unauthenticated or should we reject it
        if self
          .local_dp_sec_attributes
          .allow_unauthenticated_participants
        {
          security_log!(
            "Treating the participant with guid {:?} as Unauthenticated, since configuration \
             allows this.",
            remote_guid,
          );
          return AuthenticationStatus::Unauthenticated;
        } else {
          // Reject the damn thing
          return AuthenticationStatus::Rejected;
        }
      }
    };

    debug!(
      "Validated identity of remote participant with guid: {:?}",
      remote_guid
    );

    // Add remote participant to DiscoveryDB with status 'Authenticating' and notify
    // DP event loop. This will result in matching the builtin
    // ParticipantStatelessMessage endpoints, which are used for exchanging
    // authentication messages.
    self.add_participant_to_db_for_authentication_and_notify_dp(
      participant_data,
      discovery_db,
      discovery_updated_sender,
    );

    // What is the exact validation outcome?
    // The returned authentication status is from this match-statement
    match outcome {
      ValidationOutcome::PendingHandshakeRequest => {
        // We should send the handshake request
        let request_message = match self.create_handshake_request_message(discovery_db, remote_guid)
        {
          Ok(message) => message,
          Err(e) => {
            error!(
              "Failed to create a handshake request message. Reason: {}.",
              e.msg
            );
            // TODO: return something other than Rejected, so that we attempt to create the
            // handshake message later?
            return AuthenticationStatus::Rejected;
          }
        };

        // Add request message to cache of unanswered messages so that we'll try
        // resending it later if needed
        self.unanswered_authentication_messages.insert(
          remote_guid,
          UnansweredAuthenticationMessage::new(request_message.clone()),
        );

        // Send the message
        let _ = auth_msg_writer.write(request_message, None).map_err(|err| {
          error!(
            "Failed to send a handshake request message. Remote GUID: {:?}. Info: {}. Trying to \
             resend the message later.",
            remote_guid, err
          );
        });

        debug!(
          "Sent a handshake request message to remote with guid: {:?}",
          remote_guid
        );
        AuthenticationStatus::Authenticating // return value
      }
      ValidationOutcome::PendingHandshakeMessage => {
        // We should wait for the handshake request
        debug!(
          "Waiting for a handshake request from remote with guid {:?}",
          remote_guid
        );
        AuthenticationStatus::Authenticating // return value
      }
      outcome => {
        // Other outcomes should not be possible
        error!(
          "Got an unexpected outcome when validating remote identity. Validation outcome: {:?}.",
          outcome
        );
        AuthenticationStatus::Rejected // return value
      }
    }
  }

  fn add_participant_to_db_for_authentication_and_notify_dp(
    &mut self,
    participant_data: &SpdpDiscoveredParticipantData,
    discovery_db: &Arc<RwLock<DiscoveryDB>>,
    discovery_updated_sender: &mio_channel::SyncSender<DiscoveryNotificationType>,
  ) {
    // Add the participant to DiscoveryDB with 'Authenticating' status
    let remote_guid = participant_data.participant_guid;

    let mut db = discovery_db_write(discovery_db);
    db.update_participant(participant_data);
    db.update_authentication_status(remote_guid.prefix, AuthenticationStatus::Authenticating);

    // Send a notification to DP event loop to update the participant.
    // Since the authentication status is 'Authenticating', DP event loop will match
    // the DCPSParticipantStatelessMessage (and only that) which is the channel for
    // authentication messages
    send_discovery_notification(
      discovery_updated_sender,
      DiscoveryNotificationType::ParticipantUpdated {
        guid_prefix: remote_guid.prefix,
      },
    );
  }

  fn create_handshake_request_message(
    &mut self,
    discovery_db: &Arc<RwLock<DiscoveryDB>>,
    remote_guid: GUID,
  ) -> SecurityResult<ParticipantStatelessMessage> {
    // First get our own participant data from DB & serialize it as needed.
    // Data should always exist.
    let my_ser_data = discovery_db_read(discovery_db)
      .find_participant_proxy(self.local_participant_guid.prefix)
      .expect("My own participant data disappeared from DiscoveryDB")
      .to_pl_cdr_bytes(RepresentationIdentifier::PL_CDR_BE)
      .map_err(|e| security_error!("Serializing participant data failed: {e}"))?;

    // Get the handshake request token
    let handshake_request_token = get_security_plugins(&self.security_plugins)
      .begin_handshake_request(
        self.local_participant_guid,
        remote_guid,
        my_ser_data.to_vec(),
      )?;

    Ok(self.stateless_message_helper.new_message(
      self.local_participant_guid,
      None,
      remote_guid,
      "dds.sec.auth_request", // TODO: get this constant somewhere
      handshake_request_token,
    ))
  }

  pub fn resend_unanswered_authentication_messages(
    &mut self,
    auth_msg_writer: &no_key::DataWriter<ParticipantStatelessMessage>,
  ) {
    for (guid, unswered_message) in self.unanswered_authentication_messages.iter_mut() {
      match auth_msg_writer.write(unswered_message.message.clone(), None) {
        Ok(()) => {
          unswered_message.remaining_resend_times -= 1;
          debug!(
            "Resent an unanswered authentication message to remote with guid {:?}. Resending at \
             most {} more times.",
            guid, unswered_message.remaining_resend_times,
          );
        }
        Err(err) => {
          debug!(
            "Failed to resend an unanswered authentication message to remote with guid {:?}. \
             Error: {}. Retrying later.",
            guid, err
          );
        }
      }
    }
    // Remove messages with no more resends
    self
      .unanswered_authentication_messages
      .retain(|_guid, message| message.remaining_resend_times > 0);
  }
}

fn get_security_plugins(plugins_handle: &SecurityPluginsHandle) -> MutexGuard<SecurityPlugins> {
  plugins_handle
    .lock()
    .expect("Security plugins are poisoned")
}

fn discovery_db_read(discovery_db: &Arc<RwLock<DiscoveryDB>>) -> RwLockReadGuard<DiscoveryDB> {
  match discovery_db.read() {
    Ok(db) => db,
    Err(e) => panic!("DiscoveryDB is poisoned {:?}.", e),
  }
}

fn discovery_db_write(discovery_db: &Arc<RwLock<DiscoveryDB>>) -> RwLockWriteGuard<DiscoveryDB> {
  match discovery_db.write() {
    Ok(db) => db,
    Err(e) => panic!("DiscoveryDB is poisoned {:?}.", e),
  }
}

fn send_discovery_notification(
  discovery_updated_sender: &mio_channel::SyncSender<DiscoveryNotificationType>,
  dntype: DiscoveryNotificationType,
) {
  match discovery_updated_sender.send(dntype) {
    Ok(_) => (),
    Err(e) => error!("Failed to send DiscoveryNotification {e:?}"),
  }
}

// A helper to construct ParticipantStatelessMessages. Takes care of the
// sequence numbering of the messages
struct ParticipantStatelessMessageHelper {
  next_seqnum: SequenceNumber,
}

impl ParticipantStatelessMessageHelper {
  pub fn new() -> Self {
    Self {
      next_seqnum: SequenceNumber::new(1), /* Sequence numbering starts at 1 (see section 7.4.3.3
                                            * 'Contents of the ParticipantStatelessMessage' of
                                            * Security spec) */
    }
  }

  pub fn new_message(
    &mut self,
    local_participant_guid: GUID,
    related_message_opt: Option<&ParticipantStatelessMessage>,
    destination_participant_guid: GUID,
    message_class_id: &str,
    handshake_token: HandshakeMessageToken,
  ) -> ParticipantStatelessMessage {
    // See Section 7.4.3.3 Contents of the ParticipantStatelessMessage of the
    // Security specification

    let message_identity = rpc::SampleIdentity {
      writer_guid: local_participant_guid
        .from_prefix(EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_WRITER),
      sequence_number: self.next_seqnum,
    };

    let related_message_identity = if let Some(msg) = related_message_opt {
      msg.generic.message_identity
    } else {
      rpc::SampleIdentity {
        writer_guid: GUID::GUID_UNKNOWN,
        sequence_number: SequenceNumber::zero(),
      }
    };

    // Increment next sequence number
    self.next_seqnum = self.next_seqnum + SequenceNumber::new(1);

    let generic = ParticipantGenericMessage {
      message_identity,
      related_message_identity,
      destination_participant_guid,
      destination_endpoint_guid: GUID::GUID_UNKNOWN,
      source_endpoint_guid: GUID::GUID_UNKNOWN,
      message_class_id: message_class_id.to_string(),
      message_data: vec![handshake_token.data_holder],
    };

    ParticipantStatelessMessage { generic }
  }
}
