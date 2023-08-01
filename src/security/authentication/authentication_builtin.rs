mod authentication;
mod types;

// A struct implementing the built-in Authentication plugin
// See sections 8.3 and 9.3 of the Security specification (v. 1.1)
pub struct AuthenticationBuiltIn {
  todo: String,
}

impl AuthenticationBuiltIn {
  pub fn new() -> Self {
    Self {
      todo: "todo".to_string(),
    }
  }
}
