use std::collections::HashMap;

use crate::{
  security::{SecurityError, SecurityResult},
  security_error,
};
use self::{
  domain_governance_document::DomainRule, domain_participant_permissions_document::Grant,
};
use super::{AccessControl, PermissionsHandle};

mod domain_governance_document;
mod domain_participant_permissions_document;
mod s_mime_config_parser;

mod helpers;
mod local_entity_access_control;
mod participant_access_control;
mod remote_entity_access_control;
pub(in crate::security) mod types;

// A struct implementing the builtin Access control plugin
// See sections 8.4 and 9.4 of the Security specification (v. 1.1)
pub struct AccessControlBuiltin {
  domain_participant_grants_: HashMap<PermissionsHandle, Grant>,
  domain_rules_: HashMap<PermissionsHandle, DomainRule>,
  permissions_handle_counter_: u32,
}

impl AccessControl for AccessControlBuiltin {}

impl AccessControlBuiltin {
  pub fn new() -> Self {
    Self {
      domain_participant_grants_: HashMap::new(),
      domain_rules_: HashMap::new(),
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
}
