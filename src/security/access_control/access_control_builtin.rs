use std::collections::HashMap;

use bytes::Bytes;

use crate::{
  security::{authentication::IdentityHandle, SecurityError, SecurityResult},
  security_error,
};
use self::{
  domain_governance_document::DomainRule, domain_participant_permissions_document::Grant,
  permissions_ca_certificate::Certificate,
  config_error::{ConfigError, other_config_error, to_config_error_other, parse_config_error, }
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

  fn read_uri(&self, uri: &str) -> Result<Bytes,ConfigError> {
    match uri.split_once(':') {
      Some(("data", content)) =>  Ok(Bytes::copy_from_slice(content.as_bytes())),
      Some(("pkcs11", _)) => Err(other_config_error("Config URI schema 'pkcs11:' not implemented.".to_owned())),
      Some(("file",path)) => {
        std::fs::read(path)
          .map_err(to_config_error_other(&format!("I/O error reading {path}")))
          .map(Bytes::from)
      }
      _ => Err(parse_config_error("Config URI must begin with 'file:' , 'data:', or 'pkcs11:'.".to_owned() )),
    }
  }

}
