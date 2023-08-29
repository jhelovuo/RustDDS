use std::cmp::Ordering;

use crate::{
  security::{
    access_control::*,
    authentication::{authentication_builtin::HandshakeInfo, *},
    certificate::*,
    config::*,
    Authentication, *,
  },
  security_error,
  structure::guid::GuidPrefix,
  QosPolicies, GUID,
};
use super::{
  AuthenticationBuiltin, BuiltinHandshakeState, LocalParticipantInfo, RemoteParticipantInfo,
};

// DDS Security spec v1.1
// Section "9.3.1 Configuration" , Table 44

const QOS_IDENTITY_CA_PROPERTY_NAME: &str = "dds.sec.auth.identity_ca";
const QOS_IDENTITY_CERTIFICATE_PROPERTY_NAME: &str = "dds.sec.auth.identity_certificate";
const QOS_PRIVATE_KEY_PROPERTY_NAME: &str = "dds.sec.auth.private_key";
const QOS_PASSWORD_PROPERTY_NAME: &str = "dds.sec.auth.password";

impl Authentication for AuthenticationBuiltin {
  // Currently only mocked
  fn validate_local_identity(
    &mut self,
    domain_id: u16,
    participant_qos: &QosPolicies,
    candidate_participant_guid: GUID,
  ) -> SecurityResult<(ValidationOutcome, IdentityHandle, GUID)> {
    // TODO 1: Verify identity certificate from PropertyQosPolicy
    //
    // Steps: (spec Table 52, row 1)
    //
    // Load config files from URI's given in PropertyQosPolicy
    // * identity_ca - This is the authority that verifies the authenticity of
    // remote (and our) permissions documents
    // * private_key - Private half of the key that we use to authenticate
    // our identity to others. Others have similar keys.
    // * password - used for decrypting private key, if it is stored encrypted.
    // * identity_certificate - document that contains our subject_name,
    // public half of our identity authentication public key. This is signed by the
    // identity_ca. The purpose of the signature is that, when we send our identity
    // certificate to other Participants, they can verify (with their copy of the CA
    // certificate) that the given public key and subject name belong together.
    // The Domain Governance document gives permissions to subject names, and
    // this binding confirms that the permissions are applicable to he holder of
    // certain public-private-key pair holders.
    //
    // Validate signature of our identity_certificate with identity_ca.
    // If it does not pass, our identity certificate is useless,
    // because others would not accept it either.
    //
    // (Should also check if the certificate has been revoked.
    // The CA certificate may have a revocation list and/or check an on-line OCSP
    // server.)
    //
    // The returned IdentityHandle must be capable of
    // * reading this participant's public key (from identity_certificate)
    // * performing verify and sign operations with this participant's private key
    // * accessing the participant GUID (candidate or adjusted??)

    //TODO: These loading code snippets are too cut-and-paste. Copied from access
    // control.
    let identity_ca = participant_qos
      .get_property(QOS_IDENTITY_CA_PROPERTY_NAME)
      .and_then(|certificate_uri| {
        read_uri(&certificate_uri).map_err(|conf_err| {
          security_error!(
            "Failed to read the identity CA certificate from {}: {:?}",
            certificate_uri,
            conf_err
          )
        })
      })
      .and_then(|certificate_contents_pem| {
        Certificate::from_pem(certificate_contents_pem).map_err(|e| security_error!("{e:?}"))
      })?;

    let identity_certificate = participant_qos
      .get_property(QOS_IDENTITY_CERTIFICATE_PROPERTY_NAME)
      .and_then(|certificate_uri| {
        read_uri(&certificate_uri).map_err(|conf_err| {
          security_error!(
            "Failed to read the DomainParticipant identity certificate from {}: {:?}",
            certificate_uri,
            conf_err
          )
        })
      })
      .and_then(|certificate_contents_pem| {
        Certificate::from_pem(certificate_contents_pem).map_err(|e| security_error!("{e:?}"))
      })?;

    /*
      let private_key = participant_qos
        .get_property(QOS_PRIVATE_KEY_PROPERTY_NAME)
        .and_then(|pem_uri| {
          self.read_uri(&pem_uri).map_err(|conf_err| {
            security_error!(
              "Failed to read the DomainParticipant identity private key from {}: {:?}",
              certificate_uri,
              conf_err
            )
          })
        })
        .and_then(|private_key_pem| {
          PrivateKey::from_pem(private_key_pem).map_err(|e| security_error!("{e:?}"))
        })?;
    */

    // TODO 2: Compute the new adjusted GUID
    let adjusted_guid = candidate_participant_guid;

    // TODO 3: Create an identity handle for the local participant and associate
    // needed info (identity token, private key, public key, GUID) with it
    let local_identity_handle = self.get_new_identity_handle();

    let identity_token = IdentityToken::dummy(); // Create also this
    let private_key: Vec<u8> = Vec::default();
    let public_key: Vec<u8> = Vec::default();
    let local_participant_info = LocalParticipantInfo {
      identity_handle: local_identity_handle,
      identity_token,
      guid: adjusted_guid,
      public_key,
      private_key,
    };

    self.local_participant_info = Some(local_participant_info);

    Ok((ValidationOutcome::Ok, local_identity_handle, adjusted_guid))
  }

  fn get_identity_token(&self, handle: IdentityHandle) -> SecurityResult<IdentityToken> {
    let local_info = self.get_local_participant_info()?;

    // Parameter handle needs to correspond to the handle of the local participant
    if handle != local_info.identity_handle {
      return Err(security_error!(
        "The given handle does not correspond to the local identity handle"
      ));
    }

    // TODO: return an identity token with actual content
    Ok(local_info.identity_token.clone())
  }

  // Currently only mocked
  fn get_identity_status_token(
    &self,
    handle: IdentityHandle,
  ) -> SecurityResult<IdentityStatusToken> {
    // TODO: actual implementation

    Ok(IdentityStatusToken::dummy())
  }

  // Currently only mocked
  fn set_permissions_credential_and_token(
    &self,
    handle: IdentityHandle,
    permissions_credential_token: PermissionsCredentialToken,
    permissions_token: PermissionsToken,
  ) -> SecurityResult<()> {
    // TODO: actual implementation

    Ok(())
  }

  // Currently only mocked
  fn validate_remote_identity(
    &mut self,
    remote_auth_request_token: Option<AuthRequestMessageToken>,
    local_identity_handle: IdentityHandle,
    remote_identity_token: IdentityToken,
    remote_participant_guidp: GuidPrefix,
  ) -> SecurityResult<(
    ValidationOutcome,
    IdentityHandle,
    Option<AuthRequestMessageToken>,
  )> {
    let local_info = self.get_local_participant_info()?;
    // Make sure local_identity_handle is actually ours
    if local_identity_handle != local_info.identity_handle {
      return Err(security_error!(
        "The parameter local_identity_handle is not the correct local handle"
      ));
    }

    // TODO 1: compare remote identity token to the local one

    // Since built-in authentication does not use AuthRequestMessageToken, we ignore
    // them completely. Always return the token as None.
    let auth_request_token = None;

    // The initial handshake state depends on the lexicographic ordering of the
    // participant GUIDs. Note that the derived Ord trait produces the required
    // lexicographic ordering.
    let handshake_state = match local_info.guid.prefix.cmp(&remote_participant_guidp) {
      Ordering::Less => {
        // Our GUID is lower than remote's. We should send the request to remote
        BuiltinHandshakeState::PendingRequestSend
      }
      Ordering::Greater => {
        // Our GUID is higher than remote's. We should wait for the request from remote
        BuiltinHandshakeState::PendingRequestMessage
      }
      Ordering::Equal => {
        // This is an error, comparing with ourself.
        return Err(security_error!("Remote GUID is equal to the local GUID"));
      }
    };

    // Get new identity handle for the remote and associate remote info with it
    let remote_identity_handle = self.get_new_identity_handle();

    let remote_info = RemoteParticipantInfo {
      guid_prefix: remote_participant_guidp,
      identity_token: remote_identity_token,
      handshake: HandshakeInfo {
        state: handshake_state,
        latest_sent_message: None,
        challenge1: None,
        challenge2: None,
        shared_secret: None,
      },
    };
    self
      .remote_participant_infos
      .insert(remote_identity_handle, remote_info);

    // Map the handshake state to validation outcome of the plugin interface
    let validation_outcome = match handshake_state {
      BuiltinHandshakeState::PendingRequestSend => ValidationOutcome::PendingHandshakeRequest,
      BuiltinHandshakeState::PendingRequestMessage => ValidationOutcome::PendingHandshakeMessage,
      _ => {
        // Should not be anything else. Panic so we don't continue with inconsistent
        // authentication state
        panic!("Internal plugin error - unexpected handshake state");
      }
    };

    Ok((
      validation_outcome,
      remote_identity_handle,
      auth_request_token,
    ))
  }

  // Currently only mocked
  fn begin_handshake_request(
    &mut self,
    initiator_identity_handle: IdentityHandle, // Local
    replier_identity_handle: IdentityHandle,   // Remote
    serialized_local_participant_data: Vec<u8>,
  ) -> SecurityResult<(ValidationOutcome, HandshakeHandle, HandshakeMessageToken)> {
    // Make sure initiator_identity_handle is actually ours
    let local_info = self.get_local_participant_info()?;
    if initiator_identity_handle != local_info.identity_handle {
      return Err(security_error!(
        "The parameter initiator_identity_handle is not the correct local handle"
      ));
    }

    // Make sure we are expecting to send the authentication request message
    let remote_info = self.get_remote_participant_info_mutable(&replier_identity_handle)?;
    if remote_info.handshake.state != BuiltinHandshakeState::PendingRequestSend {
      return Err(security_error!(
        "We are not expecting to send a handshake request. Handshake state: {:?}",
        remote_info.handshake.state
      ));
    }

    // TODO 1: construct the handshake request message token
    let handshake_request = HandshakeMessageToken::dummy();

    // Change handshake state to pending reply message & save the request token
    remote_info.handshake.state = BuiltinHandshakeState::PendingReplyMessage;
    remote_info.handshake.latest_sent_message = Some(handshake_request.clone());

    // Create a new handshake handle & map it to remotes identity handle
    let new_handshake_handle = self.get_new_handshake_handle();
    self
      .handshake_to_identity_handle_map
      .insert(new_handshake_handle, replier_identity_handle);

    Ok((
      ValidationOutcome::PendingHandshakeMessage,
      new_handshake_handle,
      handshake_request,
    ))
  }

  // Currently only mocked
  fn begin_handshake_reply(
    &mut self,
    handshake_message_in: HandshakeMessageToken,
    initiator_identity_handle: IdentityHandle, // Remote
    replier_identity_handle: IdentityHandle,   // Local
    serialized_local_participant_data: Vec<u8>,
  ) -> SecurityResult<(ValidationOutcome, HandshakeHandle, HandshakeMessageToken)> {
    // Make sure replier_identity_handle is actually ours
    let local_info = self.get_local_participant_info()?;
    if replier_identity_handle != local_info.identity_handle {
      return Err(security_error!(
        "The parameter replier_identity_handle is not the correct local handle"
      ));
    }

    // Make sure we are expecting a authentication request from remote
    let remote_info = self.get_remote_participant_info_mutable(&initiator_identity_handle)?;
    if remote_info.handshake.state != BuiltinHandshakeState::PendingRequestMessage {
      return Err(security_error!(
        "We are not expecting to receive a handshake request. Handshake state: {:?}",
        remote_info.handshake.state
      ));
    }

    // TODO: Check content of authentication request tokens?

    // TODO: Verify the validity of the IdentityCredential in handshake_message_in

    // TODO: check ocsp_status / determine status of IdentityCredential another way

    // TODO: Verify the remote GUID (hash etc.)

    // TODO: Generate a proper reply token
    let reply_token = HandshakeMessageToken::dummy();

    // Change handshake state to pending final message & save the reply token
    remote_info.handshake.state = BuiltinHandshakeState::PendingFinalMessage;
    remote_info.handshake.latest_sent_message = Some(reply_token.clone());

    // Create a new handshake handle & map it to remotes identity handle
    let new_handshake_handle = self.get_new_handshake_handle();
    self
      .handshake_to_identity_handle_map
      .insert(new_handshake_handle, initiator_identity_handle);

    Ok((
      ValidationOutcome::PendingHandshakeMessage,
      new_handshake_handle,
      reply_token,
    ))
  }

  fn process_handshake(
    &mut self,
    handshake_message_in: HandshakeMessageToken,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<(ValidationOutcome, Option<HandshakeMessageToken>)> {
    // Check what is the handshake state
    let remote_identity_handle = self.handshake_handle_to_identity_handle(&handshake_handle)?;
    let remote_info = self.get_remote_participant_info(remote_identity_handle)?;

    match remote_info.handshake.state {
      BuiltinHandshakeState::PendingReplyMessage => {
        // If this passes, we get a validation outcome and a message token
        let (outcome, token) =
          self.process_handshake_reply_message(handshake_message_in, handshake_handle)?;
        Ok((outcome, Some(token)))
      }
      BuiltinHandshakeState::PendingFinalMessage => {
        // If this passes, we get an outcome only
        let outcome =
          self.process_handshake_final_message(handshake_message_in, handshake_handle)?;
        Ok((outcome, None))
      }
      other_state => {
        // Unexpected state
        Err(security_error!(
          "Unexpected handshake state: {:?}",
          other_state
        ))
      }
    }
  }

  fn get_shared_secret(
    &self,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<SharedSecretHandle> {
    let identity_handle = self.handshake_handle_to_identity_handle(&handshake_handle)?;
    let remote_info = self.get_remote_participant_info(identity_handle)?;

    let shared_secret = remote_info
      .handshake
      .shared_secret
      .clone()
      .ok_or_else(|| security_error("Shared secret not found"))?;

    let challenge1 = remote_info
      .handshake
      .challenge1
      .clone()
      .ok_or_else(|| security_error("challenge1 not found"))?;

    let challenge2 = remote_info
      .handshake
      .challenge2
      .clone()
      .ok_or_else(|| security_error("challenge2 not found"))?;

    Ok(SharedSecretHandle {
      shared_secret,
      challenge1,
      challenge2,
    })
  }

  // Currently only mocked
  fn get_authenticated_peer_credential_token(
    &self,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<AuthenticatedPeerCredentialToken> {
    // TODO: actual implementation

    Ok(AuthenticatedPeerCredentialToken::dummy())
  }

  fn set_listener(&self) -> SecurityResult<()> {
    todo!();
  }
}
