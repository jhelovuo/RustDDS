use crate::security::SecurityError;

// ValidationOutcome is like ValidationResult_t in the the Security
// specification v.1.1 (section 8.3.2.11.1), but does not contain
// VALIDATION_FAILED. Failure is handled as an error in the result type
// ValidationResult
pub enum ValidationOutcome {
  ValidationOk,
  ValidationPendingRetry,
  ValidationPendingHandshakeRequest,
  ValidationPendingHandshakeMessage,
  ValidationOkFinalMessage,
}

pub type ValidationResult = std::result::Result<ValidationOutcome, SecurityError>;

// TODO: IdentityHandle: section 8.3.2.3 of the Security specification (v. 1.1)
pub struct IdentityHandle {}
