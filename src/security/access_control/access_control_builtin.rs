use std::{collections::HashMap, ops::Not};

use bytes::Bytes;

use crate::{
  security::{authentication::IdentityHandle, SecurityError, SecurityResult},
  security_error,
};
use self::{
  config_error::{other_config_error, parse_config_error, to_config_error_other, ConfigError},
  domain_governance_document::{DomainRule, TopicRule},
  domain_participant_permissions_document::{Action, Grant},
  permissions_ca_certificate::Certificate,
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
  domain_participant_grants_: HashMap<PermissionsHandle, Grant>,
  domain_rules_: HashMap<PermissionsHandle, DomainRule>,
  permissions_ca_certificates_: HashMap<PermissionsHandle, Certificate>,
  identity_to_permissions_: HashMap<IdentityHandle, PermissionsHandle>,
  permissions_handle_counter_: u32,
}

impl AccessControl for AccessControlBuiltin {}

impl AccessControlBuiltin {
  pub fn new() -> Self {
    Self {
      domain_participant_grants_: HashMap::new(),
      domain_rules_: HashMap::new(),
      permissions_ca_certificates_: HashMap::new(),
      identity_to_permissions_: HashMap::new(),
      permissions_handle_counter_: 0,
    }
  }

  fn generate_permissions_handle_(&mut self) -> PermissionsHandle {
    self.permissions_handle_counter_ += 1;
    self.permissions_handle_counter_
  }

  fn get_domain_rule_(
    &self,
    permissions_handle: &PermissionsHandle,
  ) -> SecurityResult<&DomainRule> {
    self
      .domain_rules_
      .get(permissions_handle)
      .ok_or(security_error!(
        "Could not find a domain rule for the PermissionsHandle {}",
        permissions_handle
      ))
  }

  fn get_grant_(&self, permissions_handle: &PermissionsHandle) -> SecurityResult<&Grant> {
    self
      .domain_participant_grants_
      .get(permissions_handle)
      .ok_or(security_error!(
        "Could not find a grant for the PermissionsHandle {}",
        permissions_handle
      ))
  }

  fn get_permissions_ca_certificate_(
    &self,
    permissions_handle: &PermissionsHandle,
  ) -> SecurityResult<&Certificate> {
    self
      .permissions_ca_certificates_
      .get(permissions_handle)
      .ok_or(security_error!(
        "Could not find a permissions CA certificate for the PermissionsHandle {}",
        permissions_handle
      ))
  }

  fn get_permissions_handle_(
    &self,
    identity_handle: &IdentityHandle,
  ) -> SecurityResult<&PermissionsHandle> {
    self
      .identity_to_permissions_
      .get(identity_handle)
      .ok_or(security_error!(
        "Could not find a PermissionsHandle for the IdentityHandle {}",
        identity_handle
      ))
  }

  fn read_uri(&self, uri: &str) -> Result<Bytes, ConfigError> {
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
    // TODO: remove after testing
    if true {
      return Ok(());
    }

    let grant = self.get_grant_(&permissions_handle)?;
    let domain_rule = self.get_domain_rule_(&permissions_handle)?;

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
