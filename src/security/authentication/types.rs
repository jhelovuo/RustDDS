use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};
use ring::digest;

use crate::{
  create_security_error_and_log,
  security::{types::DataHolder, SecurityError, SecurityResult},
};

// Some generic message class IDs for authentication (see section 7.4.3.5 of the
// Security spec)
//pub const GMCLASSID_SECURITY_AUTH_REQUEST: &str = "dds.sec.auth_request"; //
// apparently never used?
pub const GMCLASSID_SECURITY_AUTH_HANDSHAKE: &str = "dds.sec.auth";

// ValidationOutcome is like ValidationResult_t in the the Security
// specification v.1.1 (section 8.3.2.11.1), but does not contain
// VALIDATION_FAILED. Failure is handled as an error in the result type
// SecurityResult
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationOutcome {
  Ok,
  // PendingRetry,  // Not included because builtin plugins never return this
  PendingHandshakeRequest,
  PendingHandshakeMessage,
  OkFinalMessage,
}

// Handles used by the authentication plugin: opaque local references to
// internal state within the AuthenticationPlugin (sec. 8.3.2 of the Security
// specification)
pub type IdentityHandle = u32;
pub type HandshakeHandle = u32;

// Wrapper for a SHA-256 hash, to avoid confusion
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Sha256([u8; 32]);

impl Sha256 {
  pub fn dummy() -> Self {
    Sha256(<[u8; 32]>::default())
  }

  pub const fn len() -> usize {
    32
  }

  pub fn hash(input: &[u8]) -> Self {
    // The .unwrap() will always succeed, because data length is
    // actually fixed to 256 / 8 = 32 bytes
    Sha256(
      digest::digest(&digest::SHA256, input)
        .as_ref()
        .try_into()
        .unwrap(),
    )
  }
}

// Ensure our idea of the length agrees with the ring
static_assertions::const_assert!(Sha256::len() == digest::SHA256_OUTPUT_LEN);

impl AsRef<[u8]> for Sha256 {
  fn as_ref(&self) -> &[u8] {
    self.0.as_ref()
  }
}

impl From<[u8; 32]> for Sha256 {
  fn from(s: [u8; 32]) -> Sha256 {
    Sha256(s)
  }
}

impl From<Sha256> for [u8; 32] {
  fn from(s: Sha256) -> [u8; 32] {
    s.0
  }
}

impl TryFrom<&[u8]> for Sha256 {
  type Error = SecurityError;
  fn try_from(v: &[u8]) -> SecurityResult<Sha256> {
    v.try_into()
      .map_err(|e| create_security_error_and_log!("Cannot read SHA-256 hash: {e:?}"))
      .map(Sha256)
  }
}

// Shared secret resulting from successful handshake
// This is a SHA256 hash of D-H key agreement result
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SharedSecret([u8; 32]);

impl SharedSecret {
  pub fn dummy() -> Self {
    SharedSecret(<[u8; 32]>::default())
  }
}

impl AsRef<[u8]> for SharedSecret {
  fn as_ref(&self) -> &[u8] {
    self.0.as_ref()
  }
}

impl From<[u8; 32]> for SharedSecret {
  fn from(s: [u8; 32]) -> SharedSecret {
    SharedSecret(s)
  }
}

impl From<Sha256> for SharedSecret {
  fn from(s: Sha256) -> SharedSecret {
    SharedSecret(s.into())
  }
}

impl From<SharedSecret> for [u8; 32] {
  fn from(s: SharedSecret) -> [u8; 32] {
    s.0
  }
}

impl TryFrom<&[u8]> for SharedSecret {
  type Error = SecurityError;
  fn try_from(v: &[u8]) -> SecurityResult<SharedSecret> {
    v.try_into()
      .map_err(|e| create_security_error_and_log!("Cannot read SharedSecret: {e:?}"))
      .map(SharedSecret)
  }
}

// Crypto challenge used in authentication protocol.
// Essentially just a 256-bit number.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Challenge([u8; 32]);

impl Challenge {
  pub fn dummy() -> Self {
    Challenge(<[u8; 32]>::default())
  }
}

impl AsRef<[u8]> for Challenge {
  fn as_ref(&self) -> &[u8] {
    self.0.as_ref()
  }
}
impl From<[u8; 32]> for Challenge {
  fn from(s: [u8; 32]) -> Challenge {
    Challenge(s)
  }
}

impl From<Challenge> for [u8; 32] {
  fn from(s: Challenge) -> [u8; 32] {
    s.0
  }
}

impl TryFrom<&[u8]> for Challenge {
  type Error = SecurityError;
  fn try_from(v: &[u8]) -> SecurityResult<Challenge> {
    v.try_into()
      .map_err(|e| create_security_error_and_log!("Cannot read Challenge: {e:?}"))
      .map(Challenge)
  }
}

// This is not really much of a handle, since it contains
// the actual data. TODO: Maybe just replace this with
// SharedSecret directly?
pub struct SharedSecretHandle {
  pub shared_secret: SharedSecret,
  pub challenge1: Challenge, // 256-bit nonce, needed to generate AES keys
  pub challenge2: Challenge, // 256-bit nonce
}

// IdentityToken: section 8.3.2.1 of the Security specification (v. 1.1)
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
  pub fn class_id(&self) -> String {
    self.data_holder.class_id.clone()
  }

  // Mock value used for development
  pub fn dummy() -> Self {
    Self {
      data_holder: DataHolder::dummy(),
    }
  }
}

// IdentityStatusToken: section 8.3.2.2 of the Security specification (v.
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

// AuthRequestMessageToken: section 8.3.2.5 of the Security specification
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

// HandshakeMessageToken: section 8.3.2.6 of the Security specification
// (v. 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeMessageToken {
  pub data_holder: DataHolder,
}

impl From<DataHolder> for HandshakeMessageToken {
  fn from(value: DataHolder) -> Self {
    Self { data_holder: value }
  }
}

impl HandshakeMessageToken {
  // Mock value used for development
  pub fn dummy() -> Self {
    Self {
      data_holder: DataHolder::dummy(),
    }
  }
}

// AuthenticatedPeerCredentialToken: section 8.3.2.7 of the Security
// specification (v. 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticatedPeerCredentialToken {
  pub data_holder: DataHolder,
}

impl From<DataHolder> for AuthenticatedPeerCredentialToken {
  fn from(value: DataHolder) -> Self {
    Self { data_holder: value }
  }
}

impl AuthenticatedPeerCredentialToken {
  // Mock value used for development
  pub fn dummy() -> Self {
    Self {
      data_holder: DataHolder::dummy(),
    }
  }
}
