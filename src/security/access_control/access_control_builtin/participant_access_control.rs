use std::ops::Not;

use chrono::Utc;

use crate::{
  dds::qos::QosPolicies,
  security::{
    access_control::{
      access_control_builtin::{
        domain_governance_document::DomainGovernanceDocument,
        domain_participant_permissions_document::DomainParticipantPermissions,
        permissions_ca_certificate::Certificate,
      },
      *,
    },
    authentication::{
      authentication_builtin::types::{
        AUTHENTICATED_PEER_TOKEN_IDENTITY_CERTIFICATE_PROPERTY_NAME,
        AUTHENTICATED_PEER_TOKEN_PERMISSIONS_DOCUMENT_PROPERTY_NAME, CERT_SN_PROPERTY_NAME,
      },
      *,
    },
    *,
  },
  security_error,
};
use super::{
  domain_governance_document::{DomainRule, TopicRule},
  s_mime_config_parser::SignedDocument,
  types::{
    BuiltinPermissionsCredentialToken, BuiltinPermissionsToken,
    BuiltinPluginParticipantSecurityAttributes,
  },
  AccessControlBuiltin,
};

const QOS_PERMISSIONS_CERTIFICATE_PROPERTY_NAME: &str = "dds.sec.access.permissions_ca";
const QOS_GOVERNANCE_DOCUMENT_PROPERTY_NAME: &str = "dds.sec.access.governance";
const QOS_PERMISSIONS_DOCUMENT_PROPERTY_NAME: &str = "dds.sec.access.permissions";

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

    // TODO remove after testing
    if true {
      return Ok(self.generate_permissions_handle_());
    }

    let permissions_ca_certificate = participant_qos
      .get_property(QOS_PERMISSIONS_CERTIFICATE_PROPERTY_NAME)
      .and_then(|certificate_uri| {
        // TODO read file
        if true {
          Ok(String::from(""))
        } else {
          Err(security_error!(
            "Failed to read the permissions certificate file {}",
            certificate_uri
          ))
        }
      })
      .and_then(|certificate_contents_pem| {
        Certificate::from_pem(certificate_contents_pem).map_err(|e| security_error!("{e:?}"))
      })?;

    let domain_rule = participant_qos
      .get_property(QOS_GOVERNANCE_DOCUMENT_PROPERTY_NAME)
      .and_then(|governance_uri| {
        // TODO read XML
        if true {
          Ok(Vec::<u8>::new())
        } else {
          Err(security_error!(
            "Failed to read the domain governance document file {}",
            governance_uri
          ))
        }
      })
      .and_then(|governance_bytes| {
        SignedDocument::from_bytes(&governance_bytes)
          .and_then(|signed_document| signed_document.verify_signature(&permissions_ca_certificate))
          .map_err(|e| security_error!("{e:?}"))
      })
      .and_then(|governance_xml| {
        DomainGovernanceDocument::from_xml(&String::from_utf8_lossy(governance_xml.as_ref()))
          .map_err(|e| security_error!("{e:?}"))
      })
      .and_then(|domain_governance_document| {
        domain_governance_document
          .find_rule(domain_id)
          .ok_or_else(|| security_error!("Domain rule not found for the domain_id {}", domain_id))
          .cloned()
      })?;

    let subject_name = auth_plugin
        .get_identity_token(identity_handle)
        .and_then(|identity_token| {
          identity_token
            .data_holder
            .get_property(CERT_SN_PROPERTY_NAME)
        })
        //TODO Parse into x509_certificate::rfc3280::Name?
        ?;

    let domain_participant_grant = participant_qos
      .get_property(QOS_PERMISSIONS_DOCUMENT_PROPERTY_NAME)
      .and_then(|permissions_uri| {
        // TODO read XML
        if true {
          Ok(Vec::<u8>::new())
        } else {
          Err(security_error!(
            "Failed to read the domain participant permissions file {}",
            permissions_uri
          ))
        }
      })
      .and_then(|permissions_bytes| {
        SignedDocument::from_bytes(&permissions_bytes)
          .and_then(|signed_document| signed_document.verify_signature(&permissions_ca_certificate))
          .map_err(|e| security_error!("{e:?}"))
      })
      .and_then(|permissions_xml| {
        DomainParticipantPermissions::from_xml(&String::from_utf8_lossy(permissions_xml.as_ref()))
          .map_err(|e| security_error!("{e:?}"))
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
    self
      .identity_to_permissions_
      .insert(identity_handle, permissions_handle);
    self
      .permissions_ca_certificates_
      .insert(permissions_handle, permissions_ca_certificate);
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

    // TODO remove after testing
    if true {
      return Ok(self.generate_permissions_handle_());
    }

    let local_permissions_handle = self.get_permissions_handle_(&local_identity_handle)?;

    let permissions_ca_certificate =
      self.get_permissions_ca_certificate_(local_permissions_handle)?;

    let remote_subject_name = remote_credential_token
      .data_holder
      .get_property(AUTHENTICATED_PEER_TOKEN_IDENTITY_CERTIFICATE_PROPERTY_NAME)
      .and_then(|certificate_contents_pem| {
        Certificate::from_pem(certificate_contents_pem).map_err(|e| security_error!("{e:?}"))
      })
      .map(|remote_identity_certificate| {
        remote_identity_certificate.subject_name();
        // TODO deal with the structured subject name
        "TODO"
      })?;

    let remote_grant = remote_credential_token
      .data_holder
      .get_property(AUTHENTICATED_PEER_TOKEN_PERMISSIONS_DOCUMENT_PROPERTY_NAME)
      .and_then(|remote_permissions_document| {
        SignedDocument::from_bytes(remote_permissions_document.as_ref())
          .and_then(|signed_document| signed_document.verify_signature(permissions_ca_certificate))
          .map_err(|e| security_error!("{e:?}"))
      })
      .and_then(|permissions_xml| {
        DomainParticipantPermissions::from_xml(&String::from_utf8_lossy(permissions_xml.as_ref()))
          .map_err(|e| security_error!("{e:?}"))
      })
      .and_then(|domain_participant_permissions| {
        domain_participant_permissions
          .find_grant(remote_subject_name, &Utc::now())
          .ok_or_else(|| {
            security_error!(
              "No valid grants with the subject name {} found",
              remote_subject_name
            )
          })
          .cloned()
      })?;

    let domain_rule = self.get_domain_rule_(local_permissions_handle).cloned()?;

    let permissions_handle = self.generate_permissions_handle_();
    self.domain_rules_.insert(permissions_handle, domain_rule);
    self
      .domain_participant_grants_
      .insert(permissions_handle, remote_grant);
    Ok(permissions_handle)
  }

  fn check_create_participant(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    qos: &QosPolicies,
  ) -> SecurityResult<()> {
    // TODO: remove after testing
    if true {
      return Ok(());
    }

    self
      // Check that permissions have been configured (is this necessary?)
      .get_grant_(&permissions_handle)
      // Get domain rule
      .and(self.get_domain_rule_(&permissions_handle))
      .and_then(
        |DomainRule {
           // corresponds to is_access_protected
           enable_join_access_control,
           topic_access_rules,
           ..
         }| {
          // Check if there is a joinable topic
          topic_access_rules
            .iter()
            .fold(
              *enable_join_access_control,
              |accumulator,
               TopicRule {
                 enable_read_access_control,
                 enable_write_access_control,
                 ..
               }| {
                accumulator && *enable_read_access_control && *enable_write_access_control
              },
            )
            .not()
            // Convert to Result
            .then_some(())
            .ok_or_else(|| {
              security_error!(
                "The participant is not allowed to join any topic by the domain rule."
              )
            })
        },
      )
    // TODO the specification seems to have a mistake here. See https://issues.omg.org/issues/DDSSEC12-79 and fix when 1.2 comes out
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

  // Currently only mocked, but ready after removing the last line
  fn get_participant_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
  ) -> SecurityResult<ParticipantSecurityAttributes> {
    self
      .get_domain_rule_(&permissions_handle)
      .map(
        |DomainRule {
           allow_unauthenticated_participants,
           enable_join_access_control,
           discovery_protection_kind,
           liveliness_protection_kind,
           rtps_protection_kind,
           ..
         }| {
          let (is_rtps_protected, is_rtps_encrypted, is_rtps_origin_authenticated) =
            rtps_protection_kind.to_security_attributes_format();
          let (is_discovery_protected, is_discovery_encrypted, is_discovery_origin_authenticated) =
            discovery_protection_kind.to_security_attributes_format();
          let (
            is_liveliness_protected,
            is_liveliness_encrypted,
            is_liveliness_origin_authenticated,
          ) = liveliness_protection_kind.to_security_attributes_format();

          ParticipantSecurityAttributes {
            allow_unauthenticated_participants: *allow_unauthenticated_participants,
            is_access_protected: *enable_join_access_control,
            is_discovery_protected,
            is_liveliness_protected,
            is_rtps_protected,
            plugin_participant_attributes: BuiltinPluginParticipantSecurityAttributes {
              is_discovery_encrypted,
              is_discovery_origin_authenticated,
              is_liveliness_encrypted,
              is_liveliness_origin_authenticated,
              is_rtps_encrypted,
              is_rtps_origin_authenticated,
            }
            .into(),
            ac_participant_properties: Vec::new(),
          }
        },
      )
      // TODO Remove after testing
      .or(Ok(ParticipantSecurityAttributes::empty()))
  }
}
