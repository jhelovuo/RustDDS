mod authentication;
pub(in crate::security) mod types;

// A struct implementing the builtin Authentication plugin
// See sections 8.3 and 9.3 of the Security specification (v. 1.1)
pub struct AuthenticationBuiltin {
  todo: String,
}

impl AuthenticationBuiltin {
  pub fn new() -> Self {
    Self {
      todo: "todo".to_string(),
    }
  }
}
