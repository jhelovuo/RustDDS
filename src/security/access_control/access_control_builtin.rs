use std::{collections::HashMap, ops::Not};

use chrono::Utc;
use bytes::Bytes;

use crate::{
  security::{authentication::IdentityHandle, SecurityError, SecurityResult},
  security_error,
};
use self::{
  config_error::{other_config_error, parse_config_error, to_config_error_other, ConfigError},
  domain_governance_document::{DomainRule, TopicRule},
  domain_participant_permissions_document::{Action, DomainParticipantPermissions, Grant},
  permissions_ca_certificate::{Certificate, DistinguishedName},
  types::Entity,
};
use super::{AccessControl, PermissionsHandle};

mod config_error;
mod domain_governance_document;
mod domain_participant_permissions_document;
mod permissions_ca_certificate;
mod s_mime_config_parser;

mod local_entity_access_control;
mod participant_access_control;
mod remote_entity_access_control;
pub(in crate::security) mod types;

// A struct implementing the builtin Access control plugin
// See sections 8.4 and 9.4 of the Security specification (v. 1.1)
pub struct AccessControlBuiltin {
  domain_participant_permissions:
    HashMap<PermissionsHandle, (DistinguishedName, DomainParticipantPermissions)>,
  domain_rules: HashMap<PermissionsHandle, DomainRule>,
  permissions_ca_certificates: HashMap<PermissionsHandle, Certificate>,
  identity_to_permissions: HashMap<IdentityHandle, PermissionsHandle>,
  permissions_handle_counter: u32,
}

impl AccessControl for AccessControlBuiltin {}

impl AccessControlBuiltin {
  pub fn new() -> Self {
    Self {
      domain_participant_permissions: HashMap::new(),
      domain_rules: HashMap::new(),
      permissions_ca_certificates: HashMap::new(),
      identity_to_permissions: HashMap::new(),
      permissions_handle_counter: 0,
    }
  }

  fn generate_permissions_handle(&mut self) -> PermissionsHandle {
    self.permissions_handle_counter += 1;
    self.permissions_handle_counter
  }

  fn get_domain_rule(&self, permissions_handle: &PermissionsHandle) -> SecurityResult<&DomainRule> {
    self.domain_rules.get(permissions_handle).ok_or_else(|| {
      security_error!(
        "Could not find a domain rule for the PermissionsHandle {}",
        permissions_handle
      )
    })
  }

  fn get_permissions_document(
    &self,
    permissions_handle: &PermissionsHandle,
  ) -> SecurityResult<&(DistinguishedName, DomainParticipantPermissions)> {
    self
      .domain_participant_permissions
      .get(permissions_handle)
      .ok_or_else(|| {
        security_error!(
          "Could not find a permissions document for the PermissionsHandle {}",
          permissions_handle
        )
      })
  }

  fn get_grant(&self, permissions_handle: &PermissionsHandle) -> SecurityResult<&Grant> {
    self.get_permissions_document(permissions_handle).and_then(
      |(subject_name, permissions_document)| {
        permissions_document
          .find_grant(subject_name, &Utc::now())
          .ok_or_else(|| {
            security_error!(
              "Could not find a valid grant for the PermissionsHandle {}",
              permissions_handle
            )
          })
      },
    )
  }

  fn get_permissions_document_string(
    &self,
    permissions_handle: &PermissionsHandle,
  ) -> SecurityResult<&String> {
    self.get_permissions_document(permissions_handle).map(
      |(
        _,
        DomainParticipantPermissions {
          original_string, ..
        },
      )| original_string,
    )
  }

  fn get_permissions_ca_certificate(
    &self,
    permissions_handle: &PermissionsHandle,
  ) -> SecurityResult<&Certificate> {
    self
      .permissions_ca_certificates
      .get(permissions_handle)
      .ok_or_else(|| {
        security_error!(
          "Could not find a permissions CA certificate for the PermissionsHandle {}",
          permissions_handle
        )
      })
  }

  fn get_permissions_handle(
    &self,
    identity_handle: &IdentityHandle,
  ) -> SecurityResult<&PermissionsHandle> {
    self
      .identity_to_permissions
      .get(identity_handle)
      .ok_or_else(|| {
        security_error!(
          "Could not find a PermissionsHandle for the IdentityHandle {}",
          identity_handle
        )
      })
  }

  // check_create_ and check_remote_ methods are very similar
  fn check_entity(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: &str,
    partitions: &[&str],
    data_tags: &[(&str, &str)],
    entity_kind: &Entity,
  ) -> SecurityResult<()> {
    let grant = self.get_grant(&permissions_handle)?;
    let domain_rule = self.get_domain_rule(&permissions_handle)?;

    let requested_access_is_unprotected = domain_rule
      .find_topic_rule(topic_name)
      .map(
        |TopicRule {
           enable_read_access_control,
           enable_write_access_control,
           ..
         }| match entity_kind {
          Entity::Datawriter => *enable_write_access_control,
          Entity::Datareader => *enable_read_access_control,
          Entity::Topic => *enable_read_access_control && *enable_write_access_control,
        },
      )
      .is_some_and(bool::not);

    let participant_has_write_access = grant
      .check_action(
        Action::Publish,
        domain_id,
        topic_name,
        partitions,
        data_tags,
      )
      .into();

    let participant_has_read_access = grant
      .check_action(
        Action::Subscribe,
        domain_id,
        topic_name,
        partitions,
        data_tags,
      )
      .into();

    let participant_has_requested_access = match entity_kind {
      Entity::Datawriter => participant_has_write_access,
      Entity::Datareader => participant_has_read_access,
      Entity::Topic => participant_has_write_access || participant_has_read_access,
    };

    (requested_access_is_unprotected || participant_has_requested_access)
      .then_some(())
      .ok_or_else(|| {
        security_error!(
          "The participant has no {} access to the topic.",
          match entity_kind {
            Entity::Datawriter => "write",
            Entity::Datareader => "read",
            Entity::Topic => "write nor read",
          }
        )
      })
  }
}

fn read_uri(uri: &str) -> Result<Bytes, ConfigError> {
  match uri.split_once(':') {
    Some(("data", content)) => Ok(Bytes::copy_from_slice(content.as_bytes())),
    Some(("pkcs11", _)) => Err(other_config_error(
      "Config URI schema 'pkcs11:' not implemented.".to_owned(),
    )),
    Some(("file", path)) => std::fs::read(path)
      .map_err(to_config_error_other(&format!("I/O error reading {path}")))
      .map(Bytes::from),
    _ => Err(parse_config_error(
      "Config URI must begin with 'file:' , 'data:', or 'pkcs11:'.".to_owned(),
    )),
  }
}
