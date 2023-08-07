mod access_control;
pub(in crate::security) mod types;

// A struct implementing the built-in Access control plugin
// See sections 8.4 and 9.4 of the Security specification (v. 1.1)
pub struct AccessControlBuiltIn {
  todo: String,
}

impl AccessControlBuiltIn {
  pub fn new() -> Self {
    Self {
      todo: "todo".to_string(),
    }
  }
}
