// This module contains empty stubs that are used
// when security feature is not in use.

// These empty types are used to avoid changing function signatures
// depending on security enabled status.

pub mod security_plugins {
  #[derive(Debug,Clone)]
  pub struct SecurityPluginsHandle {}

  impl SecurityPluginsHandle {
    pub fn new() -> Self  { SecurityPluginsHandle{} }
  }
}
pub use security_plugins::SecurityPluginsHandle;

pub struct EndpointSecurityInfo {}

pub struct SecureDiscovery {}

