use chrono::Utc;

use crate::{
  dds::qos::QosPolicies,
  security::{
    access_control::{
      access_control_builtin::{
        domain_governance_document::DomainGovernanceDocument,
        domain_participant_permissions_document::DomainParticipantPermissions,
        helpers::get_property,
      },
      *,
    },
    authentication::*,
    *,
  },
  security_error,
};
use super::{
  types::{BuiltinPermissionsCredentialToken, BuiltinPermissionsToken},
  AccessControlBuiltin,
};

// 9.4.3
impl ParticipantAccessControl for AccessControlBuiltin {
  // Currently only mocked
  fn validate_local_permissions(
    &mut self,
    auth_plugin: &dyn Authentication,
    identity_handle: IdentityHandle,
    domain_id: u16,
    participant_qos: &QosPolicies,
  ) -> SecurityResult<PermissionsHandle> {
    // TODO: actual implementation
    if true {
      return Ok(self.generate_permissions_handle_());
    }

    let domain_rule = participant_qos
      .get_property("dds.sec.access.governance")
      .and_then(|governance_uri| {
        // TODO read XML
        // TODO verify signature
        if true {
          Ok("")
        } else {
          Err(security_error!(
            "Failed to read the domain governance document file {}",
            governance_uri
          ))
        }
      })
      .and_then(|governance_xml| {
        DomainGovernanceDocument::from_xml(governance_xml).map_err(|e| security_error!("{}", e))
      })
      .and_then(|domain_governance_document| {
        domain_governance_document
          .find_rule(domain_id)
          .ok_or_else(|| security_error!("Domain rule not found for the domain_id {}", domain_id))
          .cloned()
      })?;

    let subject_name =
      auth_plugin
        .get_identity_token(identity_handle)
        .and_then(|identity_token| {
          get_property(&identity_token.data_holder.properties, "dds.cert.sn")
        })?;

    let domain_participant_grant = participant_qos
      .get_property("dds.sec.access.permissions")
      .and_then(|permissions_uri| {
        // TODO read XML
        // TODO verify signature
        if true {
          Ok("")
        } else {
          Err(security_error!(
            "Failed to read the domain participant permissions file {}",
            permissions_uri
          ))
        }
      })
      .and_then(|permissions_xml| {
        DomainParticipantPermissions::from_xml(permissions_xml)
          .map_err(|e| security_error!("{}", e))
      })
      .and_then(|domain_participant_permissions| {
        domain_participant_permissions
          .find_grant(&subject_name, &Utc::now())
          .ok_or_else(|| {
            security_error!(
              "No valid grants with the subject name {} found",
              subject_name
            )
          })
          .cloned()
      })?;

    let permissions_handle = self.generate_permissions_handle_();
    self.domain_rules_.insert(permissions_handle, domain_rule);
    self
      .domain_participant_grants_
      .insert(permissions_handle, domain_participant_grant);
    Ok(permissions_handle)
  }

  // Currently only mocked
  fn validate_remote_permissions(
    &mut self,
    auth_plugin: &dyn Authentication,
    local_identity_handle: IdentityHandle,
    remote_identity_handle: IdentityHandle,
    remote_permissions_token: PermissionsToken,
    remote_credential_token: AuthenticatedPeerCredentialToken,
  ) -> SecurityResult<PermissionsHandle> {
    // TODO: actual implementation

    todo!()
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

    Ok(
      BuiltinPermissionsToken {
        permissions_ca_subject_name: None, // TODO
        permissions_ca_algorithm: None,    // TODO
      }
      .into(),
    )
  }

  // Currently only mocked
  fn get_permissions_credential_token(
    &self,
    handle: PermissionsHandle,
  ) -> SecurityResult<PermissionsCredentialToken> {
    // TODO: actual implementation

    Ok(
      BuiltinPermissionsCredentialToken {
        permissions_certificate: "TODO".into(), // TODO
      }
      .into(),
    )
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
