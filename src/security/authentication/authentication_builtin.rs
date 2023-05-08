use crate::{dds::qos::QosPolicies, structure::guid::GUID};
use super::*;

// A struct implementing the built-in Authentication plugin
// See sections 8.3 and 9.3 of the Security specification (v. 1.1)
pub struct AuthenticationBuiltIn {
  todo: String,
}

impl AuthenticationBuiltIn {
  pub fn validate_local_identity(
    &mut self,
    local_indentity_handle: &mut IdentityHandle,
    adjusted_participant_guid: &mut GUID,
    domain_id: u16,
    participant_qos: &QosPolicies,
    candidate_participant_guid: GUID,
  ) -> ValidationResult {
    todo!();
  }
}
