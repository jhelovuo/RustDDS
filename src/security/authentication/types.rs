use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};

use crate::security::types::DataHolder;

// ValidationOutcome is like ValidationResult_t in the the Security
// specification v.1.1 (section 8.3.2.11.1), but does not contain
// VALIDATION_FAILED. Failure is handled as an error in the result type
// SecurityResult
pub enum ValidationOutcome {
  Ok,
  PendingRetry,
  PendingHandshakeRequest,
  PendingHandshakeMessage,
  OkFinalMessage,
}

// TODO: IdentityToken: section 8.3.2.1 of the Security specification (v. 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Readable, Writable)]
pub struct IdentityToken {
  // TODO: Readable & Writable are now derived, but likely need to be implemented manually.
  // Readable and Writable are needed to (de)serialize to(from) ParameterList.
  // Note: The implementation has to observe CDR alignment rules.
  pub data_holder: DataHolder,
}

impl From<DataHolder> for IdentityToken {
  fn from(value: DataHolder) -> Self {
    Self { data_holder: value }
  }
}

impl IdentityToken {
  // Mock value used for development
  pub fn dummy() -> Self {
    Self {
      data_holder: DataHolder::dummy(),
    }
  }
}

// TODO: IdentityStatusToken: section 8.3.2.2 of the Security specification (v.
// 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Readable, Writable)]
pub struct IdentityStatusToken {
  // TODO: Readable & Writable are now derived, but likely need to be implemented manually.
  // Note: The implementation has to observe CDR alignment rules.
  pub data_holder: DataHolder,
}

impl From<DataHolder> for IdentityStatusToken {
  fn from(value: DataHolder) -> Self {
    Self { data_holder: value }
  }
}

impl IdentityStatusToken {
  // Mock value used for development
  pub fn dummy() -> Self {
    Self {
      data_holder: DataHolder::dummy(),
    }
  }
}

// TODO: IdentityHandle: section 8.3.2.3 of the Security specification (v. 1.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityHandle {}

impl IdentityHandle {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}

// TODO: HandshakeHandle: section 8.3.2.4 of the Security specification (v. 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeHandle {}

impl HandshakeHandle {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}

// TODO: AuthRequestMessageToken: section 8.3.2.5 of the Security specification
// (v. 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthRequestMessageToken {
  // TODO: Readable & Writable are now derived, but likely need to be implemented manually.
  // Readable and Writable are needed to (de)serialize to(from) ParameterList.
  // Note: The implementation has to observe CDR alignment rules.
  pub data_holder: DataHolder,
}

impl From<DataHolder> for AuthRequestMessageToken {
  fn from(value: DataHolder) -> Self {
    Self { data_holder: value }
  }
}

impl AuthRequestMessageToken {
  // Mock value used for development
  pub fn dummy() -> Self {
    Self {
      data_holder: DataHolder::dummy(),
    }
  }
}

// TODO: HandshakeMessageToken: section 8.3.2.6 of the Security specification
// (v. 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeMessageToken {}

impl HandshakeMessageToken {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}

// TODO: AuthenticatedPeerCredentialToken: section 8.3.2.7 of the Security
// specification (v. 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticatedPeerCredentialToken {}

impl AuthenticatedPeerCredentialToken {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}

// TODO: SharedSecretHandle: section 8.3.2.8 of the Security specification (v.
// 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedSecretHandle {}

impl SharedSecretHandle {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}
