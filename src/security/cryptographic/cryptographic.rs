use crate::security::{
  access_control::types::*, authentication::types::*, cryptographic::types::*, types::*,
};

/// CryptoKeyFactory: section 8.5.1.7 of the Security specification (v. 1.1)
pub trait CryptoKeyFactory {
  /// register_local_participant: section 8.5.1.7.1 of the Security
  /// specification (v. 1.1)
  fn register_local_participant(
    participant_identity: IdentityHandle,
    participant_permissions: PermissionsHandle,
    participant_properties: Vec<Property>,
    participant_security_attributes: ParticipantSecurityAttributes,
  ) -> SecurityResult<ParticipantCryptoHandle>;

  /// register_matched_remote_participant: section 8.5.1.7.2 of the Security
  /// specification (v. 1.1)
  fn register_matched_remote_participant(
    local_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_identity: IdentityHandle,
    remote_participant_permissions: PermissionsHandle,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<ParticipantCryptoHandle>;

  /// register_local_datawriter: section 8.5.1.7.3 of the Security specification
  /// (v. 1.1)
  fn register_local_datawriter(
    participant_crypto: ParticipantCryptoHandle,
    datawriter_properties: Vec<Property>,
    datawriter_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatawriterCryptoHandle>;

  /// register_matched_remote_datareader: section 8.5.1.7.4 of the Security
  /// specification (v. 1.1)
  fn register_matched_remote_datareader(
    local_datawriter_crypto_handle: DatawriterCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
    shared_secret: SharedSecretHandle,
    relay_only: bool,
  ) -> SecurityResult<DatareaderCryptoHandle>;

  /// register_local_datareader: section 8.5.1.7.5 of the Security specification
  /// (v. 1.1)
  fn register_local_datareader(
    participant_crypto: ParticipantCryptoHandle,
    datareader_properties: Vec<Property>,
    datareader_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatareaderCryptoHandle>;

  /// register_matched_remote_datawriter: section 8.5.1.7.6 of the Security
  /// specification (v. 1.1)
  fn register_matched_remote_datawriter(
    local_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_participant_crypt: ParticipantCryptoHandle,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<DatareaderCryptoHandle>;

  /// unregister_participant: section 8.5.1.7.7 of the Security specification
  /// (v. 1.1)
  fn unregister_participant(
    participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<()>;
  /// unregister_datawriter: section 8.5.1.7.8 of the Security specification (v.
  /// 1.1)
  fn unregister_datawriter(datawriter_crypto_handle: DatawriterCryptoHandle) -> SecurityResult<()>;
  /// unregister_datareader: section 8.5.1.7.9 of the Security specification (v.
  /// 1.1)
  fn unregister_datareader(datareader_crypto_handle: DatareaderCryptoHandle) -> SecurityResult<()>;
}

/// CryptoKeyExchange: section 8.5.1.8 of the Security specification (v. 1.1)
pub trait CryptoKeyExchange {
  /// create_local_participant_crypto_tokens: section 8.5.1.8.1 of the Security
  /// specification (v. 1.1)
  fn create_local_participant_crypto_tokens(
    local_participant_crypto_tokens: &mut Vec<ParticipantCryptoToken>,
    local_participant_crypto: ParticipantCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
  ) -> SecurityResult<()>;

  /// set_remote_participant_crypto_tokens: section 8.5.1.8.2 of the Security
  /// specification (v. 1.1)
  fn set_remote_participant_crypto_tokens(
    local_participant_crypto: ParticipantCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
    remote_participant_tokens: Vec<ParticipantCryptoToken>,
  ) -> SecurityResult<()>;

  /// create_local_datawriter_crypto_tokens: section 8.5.1.8.3 of the Security
  /// specification (v. 1.1)
  fn create_local_datawriter_crypto_tokens(
    local_datawriter_crypto_tokens: &mut Vec<DatawriterCryptoToken>,
    local_datawriter_crypto: DatawriterCryptoHandle,
    remote_datareader_crypto: DatareaderCryptoHandle,
  ) -> SecurityResult<()>;

  /// set_remote_datawriter_crypto_tokens: section 8.5.1.8.4 of the Security
  /// specification (v. 1.1)
  fn set_remote_datawriter_crypto_tokens(
    local_datareader_crypto: DatareaderCryptoHandle,
    remote_datawriter_crypto: DatawriterCryptoHandle,
    remote_datawriter_tokens: Vec<DatawriterCryptoToken>,
  ) -> SecurityResult<()>;

  /// create_local_datareader_crypto_tokens: section 8.5.1.8.5 of the Security
  /// specification (v. 1.1)
  fn create_local_datareader_crypto_tokens(
    local_datareader_crypto_tokens: &mut Vec<DatareaderCryptoToken>,
    local_datareader_crypto: DatareaderCryptoHandle,
    remote_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<()>;

  /// set_remote_datareader_crypto_tokens: section 8.5.1.8.6 of the Security
  /// specification (v. 1.1)
  fn set_remote_datareader_crypto_tokens(
    local_datawriter_crypto: DatawriterCryptoHandle,
    remote_datareader_crypto: DatareaderCryptoHandle,
    remote_datareader_tokens: Vec<DatareaderCryptoToken>,
  ) -> SecurityResult<()>;

  /// return_crypto_tokens: section 8.5.1.8.7 of the Security specification (v.
  /// 1.1)
  fn return_crypto_tokens(crypto_tokens: Vec<CryptoToken>) -> SecurityResult<()>;
}
