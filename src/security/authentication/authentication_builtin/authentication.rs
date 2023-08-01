use crate::{
  security::{access_control::*, authentication::*, Authentication, *},
  QosPolicies, GUID,
};
use super::AuthenticationBuiltIn;

impl Authentication for AuthenticationBuiltIn {
  // Currently only mocked
  fn validate_local_identity(
    &mut self,
    domain_id: u16,
    participant_qos: &QosPolicies,
    candidate_participant_guid: GUID,
  ) -> SecurityResult<(ValidationOutcome, IdentityHandle, GUID)> {
    // TODO: actual implementation

    Ok((
      ValidationOutcome::Ok,
      IdentityHandle::default(),
      candidate_participant_guid,
    ))
  }

  // Currently only mocked
  fn get_identity_token(&self, handle: IdentityHandle) -> SecurityResult<IdentityToken> {
    // TODO: actual implementation

    Ok(IdentityToken::dummy())
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
    &self,
    remote_auth_request_token: AuthRequestMessageToken,
    local_identity_handle: IdentityHandle,
    remote_identity_token: IdentityToken,
    remote_participant_guid: GUID,
  ) -> SecurityResult<(ValidationOutcome, IdentityHandle, AuthRequestMessageToken)> {
    // TODO: actual implementation

    Ok((
      ValidationOutcome::Ok,
      IdentityHandle::default(),
      AuthRequestMessageToken::dummy(),
    ))
  }

  fn begin_handshake_request(
    &self,
    initiator_identity_handle: IdentityHandle,
    replier_identity_handle: IdentityHandle,
    serialized_local_participant_data: Vec<u8>,
  ) -> SecurityResult<(ValidationOutcome, HandshakeHandle, HandshakeMessageToken)> {
    todo!();
  }

  fn begin_handshake_reply(
    &self,
    handshake_message_in: HandshakeMessageToken,
    initiator_identity_handle: IdentityHandle,
    replier_identity_handle: IdentityHandle,
    serialized_local_participant_data: Vec<u8>,
  ) -> SecurityResult<(ValidationOutcome, HandshakeHandle, HandshakeMessageToken)> {
    todo!();
  }

  fn process_handshake(
    &self,
    handshake_message_in: HandshakeMessageToken,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<(ValidationOutcome, HandshakeMessageToken)> {
    todo!();
  }

  // Currently only mocked
  fn get_shared_secret(
    &self,
    handshake_handle: HandshakeHandle,
  ) -> SecurityResult<SharedSecretHandle> {
    // TODO: actual implementation

    Ok(SharedSecretHandle::default())
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
