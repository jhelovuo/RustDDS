use super::AccessControl;

mod local_entity_access_control;
mod participant_access_control;
mod remote_entity_access_control;
pub(in crate::security) mod types;

// A struct implementing the builtin Access control plugin
// See sections 8.4 and 9.4 of the Security specification (v. 1.1)
pub struct AccessControlBuiltin {
  todo: String,
}

impl AccessControl for AccessControlBuiltin {}

impl AccessControlBuiltin {
  pub fn new() -> Self {
    Self {
      todo: "todo".to_string(),
    }
  }
}
