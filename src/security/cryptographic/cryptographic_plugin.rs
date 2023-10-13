use crate::{
  messages::submessages::{
    elements::parameter_list::ParameterList, secure_postfix::SecurePostfix,
    secure_prefix::SecurePrefix,
  },
  rtps::{Message, Submessage},
  security::{
    access_control::types::*, authentication::types::*, cryptographic::types::*, types::*,
  },
};
// Imports for doc references
#[cfg(doc)]
use crate::{messages::submessages::submessage::SecuritySubmessage, rtps::SubmessageBody};

/// CryptoKeyFactory: section 8.5.1.7 of the Security specification (v. 1.1)
pub trait CryptoKeyFactory: Send {
  /// register_local_participant: section 8.5.1.7.1 of the Security
  /// specification (v. 1.1)
  fn register_local_participant(
    &mut self,
    participant_identity: IdentityHandle,
    participant_permissions: PermissionsHandle,
    participant_properties: &[Property],
    participant_security_attributes: ParticipantSecurityAttributes,
  ) -> SecurityResult<ParticipantCryptoHandle>;

  /// register_matched_remote_participant: section 8.5.1.7.2 of the Security
  /// specification (v. 1.1)
  fn register_matched_remote_participant(
    &mut self,
    local_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_identity: IdentityHandle,
    remote_participant_permissions: PermissionsHandle,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<ParticipantCryptoHandle>;

  /// register_local_datawriter: section 8.5.1.7.3 of the Security specification
  /// (v. 1.1)
  fn register_local_datawriter(
    &mut self,
    local_participant_crypto_handle: ParticipantCryptoHandle,
    datawriter_properties: &[Property],
    datawriter_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatawriterCryptoHandle>;

  /// register_matched_remote_datareader: section 8.5.1.7.4 of the Security
  /// specification (v. 1.1)
  fn register_matched_remote_datareader(
    &mut self,
    local_datawriter_crypto_handle: DatawriterCryptoHandle,
    remote_participant_crypto_handle: ParticipantCryptoHandle,
    shared_secret: SharedSecretHandle,
    relay_only: bool,
  ) -> SecurityResult<DatareaderCryptoHandle>;

  /// register_local_datareader: section 8.5.1.7.5 of the Security specification
  /// (v. 1.1)
  fn register_local_datareader(
    &mut self,
    local_participant_crypto_handle: ParticipantCryptoHandle,
    datareader_properties: &[Property],
    datareader_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatareaderCryptoHandle>;

  /// register_matched_remote_datawriter: section 8.5.1.7.6 of the Security
  /// specification (v. 1.1)
  fn register_matched_remote_datawriter(
    &mut self,
    local_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_participant_crypto_handle: ParticipantCryptoHandle,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<DatawriterCryptoHandle>;

  /// unregister_participant: section 8.5.1.7.7 of the Security specification
  /// (v. 1.1)
  fn unregister_participant(
    &mut self,
    participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<()>;
  /// unregister_datawriter: section 8.5.1.7.8 of the Security specification (v.
  /// 1.1)
  fn unregister_datawriter(
    &mut self,
    datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<()>;
  /// unregister_datareader: section 8.5.1.7.9 of the Security specification (v.
  /// 1.1)
  fn unregister_datareader(
    &mut self,
    datareader_crypto_handle: DatareaderCryptoHandle,
  ) -> SecurityResult<()>;
}

/// CryptoKeyExchange: section 8.5.1.8 of the Security specification (v. 1.1)
pub trait CryptoKeyExchange {
  /// create_local_participant_crypto_tokens: section 8.5.1.8.1 of the Security
  /// specification (v. 1.1)
  ///
  /// In a vector, return the tokens that would be written in
  /// `local_participant_crypto_tokens`.
  fn create_local_participant_crypto_tokens(
    &mut self,
    local_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<Vec<ParticipantCryptoToken>>;

  /// set_remote_participant_crypto_tokens: section 8.5.1.8.2 of the Security
  /// specification (v. 1.1)
  fn set_remote_participant_crypto_tokens(
    &mut self,
    local_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_tokens: Vec<ParticipantCryptoToken>,
  ) -> SecurityResult<()>;

  /// create_local_datawriter_crypto_tokens: section 8.5.1.8.3 of the Security
  /// specification (v. 1.1)
  ///
  /// In a vector, return the tokens that would be written in
  /// `local_datawriter_crypto_tokens`.
  fn create_local_datawriter_crypto_tokens(
    &mut self,
    local_datawriter_crypto_handle: DatawriterCryptoHandle,
    remote_datareader_crypto_handle: DatareaderCryptoHandle,
  ) -> SecurityResult<Vec<DatawriterCryptoToken>>;

  /// set_remote_datawriter_crypto_tokens: section 8.5.1.8.4 of the Security
  /// specification (v. 1.1)
  fn set_remote_datawriter_crypto_tokens(
    &mut self,
    local_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_datawriter_crypto_handle: DatawriterCryptoHandle,
    remote_datawriter_tokens: Vec<DatawriterCryptoToken>,
  ) -> SecurityResult<()>;

  /// create_local_datareader_crypto_tokens: section 8.5.1.8.5 of the Security
  /// specification (v. 1.1)
  ///
  /// In a vector, return the tokens that would be written in
  /// `local_datareader_crypto_tokens`.
  fn create_local_datareader_crypto_tokens(
    &mut self,
    local_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<Vec<DatareaderCryptoToken>>;

  /// set_remote_datareader_crypto_tokens: section 8.5.1.8.6 of the Security
  /// specification (v. 1.1)
  fn set_remote_datareader_crypto_tokens(
    &mut self,
    local_datawriter_crypto_handle: DatawriterCryptoHandle,
    remote_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_datareader_tokens: Vec<DatareaderCryptoToken>,
  ) -> SecurityResult<()>;

  /// return_crypto_tokens: section 8.5.1.8.7 of the Security specification (v.
  /// 1.1)
  fn return_crypto_tokens(&mut self, crypto_tokens: Vec<CryptoToken>) -> SecurityResult<()>;
}

/// CryptoTransform: section 8.5.1.9 of the Security specification (v. 1.1)
///
/// Differs from the specification by returning the results instead of writing
/// them to provided buffers.
pub trait CryptoTransform: Send {
  /// encode_serialized_payload: section 8.5.1.9.1 of the Security specification
  /// (v. 1.1)
  ///
  /// In a tuple, return the results that would be written in `encoded_buffer`,
  /// which can be `CryptoContent`, or `SerializedPayload` if no encryption is
  /// performed, and `extra_inline_qos`.
  fn encode_serialized_payload(
    &self,
    plain_buffer: Vec<u8>, // (a fragment of) serialized `SerializedPayload`
    sending_datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<(Vec<u8>, ParameterList)>;

  /// encode_datawriter_submessage: section 8.5.1.9.2 of the Security
  /// specification (v. 1.1)
  ///
  /// Return the submessages that would be written in
  /// `encoded_rtps_submessage`.
  /// `receiving_datareader_crypto_list_index` is dropped.
  ///
  /// NOTE! [crate::security::security_plugins::SecurityPlugins::is_rtps_protection_special_case] relies on the assumption that
  /// in the topic DCPSParticipantVolatileMessageSecure
  /// the CryptoTransformIdentifier has transformation_key_id=0 like it does in
  /// the builtin plugin. If a custom plugin that does not adhere to this is
  /// used, that check needs to also be modified.
  ///
  /// # Panics
  /// The function may panic if `plain_rtps_submessage.body` is not
  /// [SubmessageBody::Writer].
  fn encode_datawriter_submessage(
    &self,
    plain_rtps_submessage: Submessage,
    sending_datawriter_crypto_handle: DatawriterCryptoHandle,
    receiving_datareader_crypto_handle_list: Vec<DatareaderCryptoHandle>,
  ) -> SecurityResult<EncodedSubmessage>;

  /// encode_datareader_submessage: section 8.5.1.9.3 of the Security
  /// specification (v. 1.1)
  ///
  /// Return the submessages that would be written in
  /// `encoded_rtps_submessage`.
  ///
  /// NOTE! [crate::security::security_plugins::SecurityPlugins::is_rtps_protection_special_case] relies on the assumption that
  /// in the topic DCPSParticipantVolatileMessageSecure
  /// the CryptoTransformIdentifier has transformation_key_id=0 like it does in
  /// the builtin plugin. If a custom plugin that does not adhere to this is
  /// used, that check needs to also be modified.
  ///
  /// # Panics
  /// The function may panic if `plain_rtps_submessage.body` is not
  /// [SubmessageBody::Reader].
  fn encode_datareader_submessage(
    &self,
    plain_rtps_submessage: Submessage,
    sending_datareader_crypto_handle: DatareaderCryptoHandle,
    receiving_datawriter_crypto_handle_list: Vec<DatawriterCryptoHandle>,
  ) -> SecurityResult<EncodedSubmessage>;

  /// encode_rtps_message: section 8.5.1.9.4 of the Security specification (v.
  /// 1.1)
  ///
  /// Return the message that would be written in
  /// `encoded_rtps_message`.
  /// in the case that no transformation is performed and the plain message
  /// should be sent, the plain message shall be returned (instead of returning
  /// false, see the spec). `receiving_participant_crypto_list_index` is
  /// dropped.
  fn encode_rtps_message(
    &self,
    plain_rtps_message: Message,
    sending_participant_crypto_handle: ParticipantCryptoHandle,
    receiving_participant_crypto_handle_list: Vec<ParticipantCryptoHandle>,
  ) -> SecurityResult<Message>;

  /// decode_rtps_message: section 8.5.1.9.5 of the Security specification (v.
  /// 1.1)
  ///
  /// Return the message that would be written in `plain_buffer`.
  fn decode_rtps_message(
    &self,
    encoded_message: Message,
    receiving_participant_crypto_handle: ParticipantCryptoHandle,
    sending_participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<DecodeOutcome<Message>>;

  // Combines the functionality of preprocess_secure_submsg and the subsequent
  // call of decode_datawriter_submessage or decode_datareader_submessage from
  // sections 8.5.1.9.6–8 of the Security specification
  /// (v. 1.1)
  ///
  /// Return the body of the submessage that would be written in
  /// `plain_rtps_submessage` of the appropriate decode method, and a list of
  /// endpoint crypto handles, the decode keys of which match the one used for
  /// decoding, i.e. which are approved to receive the submessage by access
  /// control.
  fn decode_submessage(
    &self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    receiving_local_participant_crypto_handle: ParticipantCryptoHandle,
    sending_remote_participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<DecodeOutcome<DecodedSubmessage>>;

  /// decode_serialized_payload: section 8.5.1.9.9 of the Security specification
  /// (v. 1.1)
  ///
  /// Return the (fragment of) serialized payload that would be written in
  /// `plain_buffer`
  fn decode_serialized_payload(
    &self,
    encoded_buffer: Vec<u8>,
    inline_qos: ParameterList,
    receiving_datareader_crypto_handle: DatareaderCryptoHandle,
    sending_datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<Vec<u8>>;
}
