use std::collections::HashMap;

use bytes::Bytes;

use crate::{
  security::{certificate, SecurityError, SecurityResult},
  security_error,
  structure::guid::GuidPrefix,
  GUID,
};
use super::{
  authentication_builtin::types::BuiltinIdentityToken, HandshakeHandle, HandshakeMessageToken,
  IdentityHandle, IdentityToken, SharedSecret, ValidationOutcome,
};

mod authentication;
pub(in crate::security) mod types;

// States for an ongoing handshake with a remote participant. Used by the plugin
// internally. Note that there is no 'failed' state, since once a handshake has
// started, it doesn't terminate if some step fails. Instead, it just doesn't
// advance to the next step.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum BuiltinHandshakeState {
  PendingRequestSend,    // We need to create & send the handshake request
  PendingRequestMessage, // We are waiting for a handshake request from remote participant
  PendingReplyMessage,   // We have sent a handshake request and are waiting for a reply
  PendingFinalMessage,   /* We have sent a handshake reply message and are waiting for the
                          * final message */
  CompletedWithFinalMessageSent, /* Handshake was completed & we sent the final message. If
                                  * requested again, we need to resend the message */
  CompletedWithFinalMessageReceived, /* Handshake was completed & we received the final
                                      * message. Nothing to do for us anymore. */
}

struct LocalParticipantInfo {
  identity_handle: IdentityHandle,
  identity_token: BuiltinIdentityToken,
  guid: GUID,
  private_key: certificate::PrivateKey, // PrivateKey is actually (private,public) key pair
  public_key: certificate::Certificate, // Certificate contains the public key also
}

// All things about remote participant that we're interested in
struct RemoteParticipantInfo {
  identity_token: IdentityToken,
  guid_prefix: GuidPrefix,
  handshake: HandshakeInfo,
}

use x509_certificate::signing::{InMemorySigningKeyPair};

struct HandshakeInfo {
  state: BuiltinHandshakeState,
  latest_sent_message: Option<HandshakeMessageToken>,
  my_dh_keys: Option<InMemorySigningKeyPair>, // This is dh1 or dh2, whichever we generated ourself
  challenge1: Option<Bytes>,
  challenge2: Option<Bytes>,
  shared_secret: Option<SharedSecret>,
}

// A struct implementing the builtin Authentication plugin
// See sections 8.3 and 9.3 of the Security specification (v. 1.1)
pub struct AuthenticationBuiltin {
  local_participant_info: Option<LocalParticipantInfo>,
  remote_participant_infos: HashMap<IdentityHandle, RemoteParticipantInfo>,
  // handshake_to_identity_handle maps handshake handles to identity handles.
  handshake_to_identity_handle_map: HashMap<HandshakeHandle, IdentityHandle>,

  next_identity_handle: IdentityHandle,
  next_handshake_handle: HandshakeHandle,
}

impl AuthenticationBuiltin {
  pub fn new() -> Self {
    Self {
      local_participant_info: None, // No info yet
      remote_participant_infos: HashMap::new(),
      handshake_to_identity_handle_map: HashMap::new(),
      next_identity_handle: 0,
      next_handshake_handle: 0,
    }
  }

  fn get_new_identity_handle(&mut self) -> IdentityHandle {
    let new_handle = self.next_identity_handle;
    self.next_identity_handle += 1;
    new_handle
  }

  fn get_new_handshake_handle(&mut self) -> HandshakeHandle {
    let new_handle = self.next_handshake_handle;
    self.next_handshake_handle += 1;
    new_handle
  }

  fn get_local_participant_info(&self) -> SecurityResult<&LocalParticipantInfo> {
    self.local_participant_info.as_ref().ok_or_else(|| {
      security_error!("Local participant info not found. Has the local identity been validated?")
    })
  }

  // Returns inmutable info
  fn get_remote_participant_info(
    &self,
    identity_handle: &IdentityHandle,
  ) -> SecurityResult<&RemoteParticipantInfo> {
    self
      .remote_participant_infos
      .get(identity_handle)
      .ok_or_else(|| security_error!("Remote participant info not found"))
  }

  // Returns mutable info
  fn get_remote_participant_info_mutable(
    &mut self,
    identity_handle: &IdentityHandle,
  ) -> SecurityResult<&mut RemoteParticipantInfo> {
    self
      .remote_participant_infos
      .get_mut(identity_handle)
      .ok_or_else(|| security_error!("Remote participant info not found"))
  }

  fn handshake_handle_to_identity_handle(
    &self,
    hs_handle: &HandshakeHandle,
  ) -> SecurityResult<&IdentityHandle> {
    self
      .handshake_to_identity_handle_map
      .get(hs_handle)
      .ok_or_else(|| security_error!("Identity handle not found with handshake handle"))
  }

  // Currently only mocked
  #[allow(clippy::needless_pass_by_value)]
  fn process_handshake_reply_message(
    &mut self,
    reply_message: HandshakeMessageToken,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<(ValidationOutcome, HandshakeMessageToken)> {
    let remote_identity_handle = *self.handshake_handle_to_identity_handle(&handshake_handle)?;
    let remote_info = self.get_remote_participant_info_mutable(&remote_identity_handle)?;

    // TODO: verify the contents of the reply message token

    // TODO: verify the conten of authentication request token?

    // TODO: Verify validity of IdentityCredential

    // TODO: verify ocsp_status / status of IdentityCredential

    // TODO: check that challenge1 is equal to the challenge1 sent in request
    // token

    // TODO: verify the digital signature

    // TODO: store the value of property with name “dds.sec.” found within the
    // handshake_message_in

    // TODO: Compute the shared secret
    let shared_secret = SharedSecret::default();
    let challenge1 = Bytes::default();
    let challenge2 = Bytes::default();

    // TODO: Create proper HandshakeFinalMessageToken
    let final_message_token = HandshakeMessageToken::dummy();

    // Change handshake state to Completed & save the final message token
    remote_info.handshake.state = BuiltinHandshakeState::CompletedWithFinalMessageSent;
    remote_info.handshake.latest_sent_message = Some(final_message_token.clone());
    remote_info.handshake.challenge1 = Some(challenge1);
    remote_info.handshake.challenge2 = Some(challenge2);
    remote_info.handshake.shared_secret = Some(shared_secret);

    Ok((ValidationOutcome::OkFinalMessage, final_message_token))
  }

  // Currently only mocked
  #[allow(clippy::needless_pass_by_value)]
  fn process_handshake_final_message(
    &mut self,
    final_message: HandshakeMessageToken,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<ValidationOutcome> {
    let remote_identity_handle = *self.handshake_handle_to_identity_handle(&handshake_handle)?;
    let remote_info = self.get_remote_participant_info_mutable(&remote_identity_handle)?;

    // TODO: verify the contents of the final message token

    // TODO: verify matching of challenge1 and challenge2 to what we sent in the
    // reply token

    // TODO: verify the digital signature

    // TODO: compute the shared secret
    let shared_secret = SharedSecret::default();
    let challenge1 = Bytes::default();
    let challenge2 = Bytes::default();

    // Change handshake state to Completed
    remote_info.handshake.state = BuiltinHandshakeState::CompletedWithFinalMessageReceived;
    remote_info.handshake.challenge1 = Some(challenge1);
    remote_info.handshake.challenge2 = Some(challenge2);
    remote_info.handshake.shared_secret = Some(shared_secret);

    Ok(ValidationOutcome::Ok)
  }
}
