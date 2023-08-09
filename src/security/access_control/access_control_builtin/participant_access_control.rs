use crate::{
  dds::qos::QosPolicies,
  security::{access_control::*, authentication::*, *},
};
use super::AccessControlBuiltin;

impl ParticipantAccessControl for AccessControlBuiltin {
  // Currently only mocked
  fn validate_local_permissions(
    &self,
    auth_plugin: &dyn Authentication,
    identity: IdentityHandle,
    domain_id: u16,
    participant_qos: &QosPolicies,
  ) -> SecurityResult<PermissionsHandle> {
    // TODO: actual implementation

    Ok(PermissionsHandle::MOCK)
  }

  // Currently only mocked
  fn validate_remote_permissions(
    &self,
    auth_plugin: &dyn Authentication,
    local_identity_handle: IdentityHandle,
    remote_identity_handle: IdentityHandle,
    remote_permissions_token: PermissionsToken,
    remote_credential_token: AuthenticatedPeerCredentialToken,
  ) -> SecurityResult<PermissionsHandle> {
    // TODO: actual implementation

    Ok(PermissionsHandle::MOCK)
  }

  // Currently only mocked
  fn check_create_participant(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    qos: &QosPolicies,
  ) -> SecurityResult<()> {
    // TODO: actual implementation

    Ok(())
  }

  // Currently only mocked
  fn get_permissions_token(&self, handle: PermissionsHandle) -> SecurityResult<PermissionsToken> {
    // TODO: actual implementation

    Ok(PermissionsToken::MOCK)
  }

  // Currently only mocked
  fn get_permissions_credential_token(
    &self,
    handle: PermissionsHandle,
  ) -> SecurityResult<PermissionsCredentialToken> {
    // TODO: actual implementation

    Ok(PermissionsCredentialToken::MOCK)
  }

  fn set_listener(&self) -> SecurityResult<()> {
    todo!();
  }

  // Currently only mocked
  fn get_participant_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
  ) -> SecurityResult<ParticipantSecurityAttributes> {
    // TODO: actual implementation

    Ok(ParticipantSecurityAttributes::empty())
  }
}
