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

// TODO: IdentityToken: section 8.3.2.1 of the Security specification (v. 1.1)
pub struct IdentityToken {}

// TODO: IdentityStatusToken: section 8.3.2.2 of the Security specification (v. 1.1)
pub struct IdentityStatusToken {}

// TODO: IdentityHandle: section 8.3.2.3 of the Security specification (v. 1.1)
pub struct IdentityHandle {}

// TODO: HandshakeHandle: section 8.3.2.4 of the Security specification (v. 1.1)
pub struct HandshakeHandle {}

// TODO: AuthRequestMessageToken: section 8.3.2.5 of the Security specification (v. 1.1)
pub struct AuthRequestMessageToken {}

// TODO: HandshakeMessageToken: section 8.3.2.6 of the Security specification (v. 1.1)
pub struct HandshakeMessageToken {}

// TODO: AuthenticatedPeerCredentialToken: section 8.3.2.7 of the Security specification (v. 1.1)
pub struct AuthenticatedPeerCredentialToken {}

// TODO: SharedSecretHandle: section 8.3.2.8 of the Security specification (v. 1.1)
pub struct SharedSecretHandle {}