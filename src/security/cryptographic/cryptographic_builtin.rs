use speedy::Writable;

use crate::{
  messages::submessages::elements::{
    crypto_content::CryptoContent, crypto_header::CryptoHeader, parameter_list::ParameterList,
    serialized_payload::SerializedPayload,
  },
  rtps::{Message, Submessage, SubmessageBody},
  security::{
    access_control::types::*,
    authentication::types::*,
    cryptographic::{builtin_types::*, cryptographic_plugin::*, types::*},
    types::*,
  },
};

// A struct implementing the built-in Cryptographic plugin
// See sections 8.5 and 9.5 of the Security specification (v. 1.1)
pub struct CryptographicBuiltIn {}

/// Builtin CryptoKeyFactory implementation from section 9.5.3.1 of the Security
/// specification (v. 1.1)
impl CryptoKeyFactory for CryptographicBuiltIn {
  fn register_local_participant(
    participant_identity: IdentityHandle,
    participant_permissions: PermissionsHandle,
    participant_properties: Vec<Property>,
    participant_security_attributes: ParticipantSecurityAttributes,
  ) -> SecurityResult<ParticipantCryptoHandle> {
    //TODO: this is only a mock implementation
    KeyMaterial_AES_GCM_GMAC {
      transformation_kind: BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE,
      master_salt: Vec::new(),
      sender_key_id: [0, 0, 0, 0],
      master_sender_key: Vec::new(),
      receiver_specific_key_id: [0, 0, 0, 0],
      master_receiver_specific_key: Vec::new(),
    }
    .try_into()
  }

  fn register_matched_remote_participant(
    local_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_identity: IdentityHandle,
    remote_participant_permissions: PermissionsHandle,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<ParticipantCryptoHandle> {
    //TODO: this is only a mock implementation
    let KeyMaterial_AES_GCM_GMAC {
      transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      receiver_specific_key_id,
      master_receiver_specific_key,
    }: KeyMaterial_AES_GCM_GMAC = local_participant_crypto_handle.try_into()?;
    KeyMaterial_AES_GCM_GMAC {
      transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      receiver_specific_key_id: [0, 0, 0, 0],
      master_receiver_specific_key: Vec::new(),
    }
    .try_into()
  }

  fn register_local_datawriter(
    participant_crypto: ParticipantCryptoHandle,
    datawriter_properties: Vec<Property>,
    datawriter_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatawriterCryptoHandle> {
    //TODO: this is only a mock implementation
    KeyMaterial_AES_GCM_GMAC_seq(vec![KeyMaterial_AES_GCM_GMAC {
      transformation_kind: BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE,
      master_salt: Vec::new(),
      sender_key_id: [0, 0, 0, 0],
      master_sender_key: Vec::new(),
      receiver_specific_key_id: [0, 0, 0, 0],
      master_receiver_specific_key: Vec::new(),
    }])
    .try_into()
  }

  fn register_matched_remote_datareader(
    local_datawriter_crypto_handle: DatawriterCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
    shared_secret: SharedSecretHandle,
    relay_only: bool,
  ) -> SecurityResult<DatareaderCryptoHandle> {
    //TODO: this is only a mock implementation
    let writer_key_material_seq =
      KeyMaterial_AES_GCM_GMAC_seq::try_from(local_datawriter_crypto_handle)?;
    // ??? append something ???

    writer_key_material_seq.try_into()
  }

  fn register_local_datareader(
    participant_crypto: ParticipantCryptoHandle,
    datareader_properties: Vec<Property>,
    datareader_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatareaderCryptoHandle> {
    //TODO: this is only a mock implementation
    KeyMaterial_AES_GCM_GMAC_seq(vec![KeyMaterial_AES_GCM_GMAC {
      transformation_kind: BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE,
      master_salt: Vec::new(),
      sender_key_id: [0, 0, 0, 0],
      master_sender_key: Vec::new(),
      receiver_specific_key_id: [0, 0, 0, 0],
      master_receiver_specific_key: Vec::new(),
    }])
    .try_into()
  }

  fn register_matched_remote_datawriter(
    local_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_participant_crypt: ParticipantCryptoHandle,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<DatawriterCryptoHandle> {
    //TODO: this is only a mock implementation
    let KeyMaterial_AES_GCM_GMAC {
      transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      receiver_specific_key_id,
      master_receiver_specific_key,
    }: KeyMaterial_AES_GCM_GMAC = local_datareader_crypto_handle.try_into()?;
    KeyMaterial_AES_GCM_GMAC {
      transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      receiver_specific_key_id: [0, 0, 0, 0],
      master_receiver_specific_key: Vec::new(),
    }
    .try_into()
  }

  fn unregister_participant(
    participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }

  fn unregister_datawriter(datawriter_crypto_handle: DatawriterCryptoHandle) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }

  fn unregister_datareader(datareader_crypto_handle: DatareaderCryptoHandle) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }
}

impl CryptoKeyExchange for CryptographicBuiltIn {
  fn create_local_participant_crypto_tokens(
    local_participant_crypto: ParticipantCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
  ) -> SecurityResult<Vec<ParticipantCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)

    // Convert to key material
    KeyMaterial_AES_GCM_GMAC::try_from(remote_participant_crypto)
      // Convert to CryptoToken and wrap in a Vec
      .and_then(|keymat| Ok(vec![keymat.try_into()?]))
  }

  fn set_remote_participant_crypto_tokens(
    local_participant_crypto: ParticipantCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
    remote_participant_tokens: Vec<ParticipantCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }

  fn create_local_datawriter_crypto_tokens(
    local_datawriter_crypto: DatawriterCryptoHandle,
    remote_datareader_crypto: DatareaderCryptoHandle,
  ) -> SecurityResult<Vec<DatawriterCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)

    // Convert to key material seq
    KeyMaterial_AES_GCM_GMAC_seq::try_from(remote_datareader_crypto)
      // Convert to Vec<CryptoToken>
      .and_then(Vec::<DatawriterCryptoToken>::try_from)
  }

  fn set_remote_datawriter_crypto_tokens(
    local_datareader_crypto: DatareaderCryptoHandle,
    remote_datawriter_crypto: DatawriterCryptoHandle,
    remote_datawriter_tokens: Vec<DatawriterCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }

  fn create_local_datareader_crypto_tokens(
    local_datareader_crypto: DatareaderCryptoHandle,
    remote_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<Vec<DatareaderCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)

    // Convert to key material
    KeyMaterial_AES_GCM_GMAC::try_from(remote_datawriter_crypto)
      // Convert to CryptoToken and wrap in a Vec
      .and_then(|keymat| Ok(vec![DatareaderCryptoToken::try_from(keymat)?]))
  }

  fn set_remote_datareader_crypto_tokens(
    local_datawriter_crypto: DatawriterCryptoHandle,
    remote_datareader_crypto: DatareaderCryptoHandle,
    remote_datareader_tokens: Vec<DatareaderCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }

  fn return_crypto_tokens(crypto_tokens: Vec<CryptoToken>) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }
}

impl CryptoTransform for CryptographicBuiltIn {
  fn encode_serialized_payload(
    plain_buffer: SerializedPayload,
    sending_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<(CryptoContent, ParameterList)> {
    //TODO: this is only a mock implementation
    let plaintext = plain_buffer.write_to_vec().map_err(|err| SecurityError {
      msg: format!("Error converting SerializedPayload to byte vector: {}", err),
    })?;
    let KeyMaterial_AES_GCM_GMAC_seq(keymat_seq) =
      KeyMaterial_AES_GCM_GMAC_seq::try_from(sending_datawriter_crypto)?;

    match keymat_seq.as_slice() {
      [keymat_0] => {
        let header = BuiltinCryptoHeader {
          transform_identifier: BuiltinCryptoTransformIdentifier {
            transformation_kind: keymat_0.transformation_kind,
            transformation_key_id: [0, 0, 0, 0],
          },
          session_id: [0, 0, 0, 0],
          initialization_vector_suffix: [0; 8],
        };
        match keymat_0.transformation_kind {
          BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
            let footer = BuiltinCryptoFooter {
              common_mac: [0; 16],
              receiver_specific_macs: Vec::new(),
            };

            let header_vec =
              CryptoHeader::from(header)
                .write_to_vec()
                .map_err(|err| SecurityError {
                  msg: format!("Error converting CryptoHeader to byte vector: {}", err),
                })?;
            let footer_vec = Vec::<u8>::try_from(footer)?;
            Ok((
              CryptoContent::from([header_vec, plaintext, footer_vec].concat()),
              ParameterList::new(),
            ))
          }
          _ => todo!(),
        }
      }
      _ => todo!(),
    }
  }

  fn encode_datawriter_submessage(
    plain_rtps_submessage: Submessage,
    sending_datawriter_crypto: DatawriterCryptoHandle,
    receiving_datareader_crypto_list: Vec<DatareaderCryptoHandle>,
  ) -> SecurityResult<EncodeResult<EncodedSubmessage>> {
    //TODO: this is only a mock implementation
    let KeyMaterial_AES_GCM_GMAC_seq(keymat_seq) =
      KeyMaterial_AES_GCM_GMAC_seq::try_from(sending_datawriter_crypto)?;

    match keymat_seq.as_slice() {
      [keymat_0] => match keymat_0.transformation_kind {
        BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => Ok(EncodeResult::One(
          EncodedSubmessage::Unencoded(plain_rtps_submessage),
        )),
        _ => todo!(),
      },
      _ => todo!(),
    }
  }

  fn encode_datareader_submessage(
    plain_rtps_submessage: Submessage,
    sending_datareader_crypto: DatareaderCryptoHandle,
    receiving_datawriter_crypto_list: Vec<DatawriterCryptoHandle>,
  ) -> SecurityResult<EncodeResult<EncodedSubmessage>> {
    //TODO: this is only a mock implementation

    let keymat = KeyMaterial_AES_GCM_GMAC::try_from(sending_datareader_crypto)?;

    match keymat.transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => Ok(EncodeResult::One(
        EncodedSubmessage::Unencoded(plain_rtps_submessage),
      )),
      _ => todo!(),
    }
  }

  fn encode_rtps_message(
    plain_rtps_message: Message,
    sending_participant_crypto: ParticipantCryptoHandle,
    receiving_participant_crypto_list: Vec<ParticipantCryptoHandle>,
  ) -> SecurityResult<EncodeResult<Message>> {
    //TODO: this is only a mock implementation

    let keymat = KeyMaterial_AES_GCM_GMAC::try_from(sending_participant_crypto)?;

    match keymat.transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        Ok(EncodeResult::One(plain_rtps_message))
      }
      _ => todo!(),
    }
  }

  fn decode_rtps_message(
    encoded_buffer: Message,
    receiving_participant_crypto: ParticipantCryptoHandle,
    sending_participant_crypto: ParticipantCryptoHandle,
  ) -> SecurityResult<Message> {
    //TODO: this is only a mock implementation
    match encoded_buffer
      .submessages
      .first()
      .map(|submessage| submessage.body.clone())
    {
      Some(SubmessageBody::Security(_)) => todo!(),
      _ => Ok(encoded_buffer),
    }
  }

  fn preprocess_secure_submsg(
    encoded_rtps_submessage: Submessage,
    receiving_participant_crypto: ParticipantCryptoHandle,
    sending_participant_crypto: ParticipantCryptoHandle,
  ) -> SecurityResult<SecureSubmessageCategory> {
    todo!(); // 9.5.3.3.5  Compare key ID to figure out which is which?
  }

  fn decode_datawriter_submessage(
    encoded_rtps_submessage: (Submessage, Submessage, Submessage),
    receiving_datareader_crypto: DatareaderCryptoHandle,
    sending_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<Submessage> {
    todo!();
  }

  fn decode_datareader_submessage(
    encoded_rtps_submessage: (Submessage, Submessage, Submessage),
    receiving_datawriter_crypto: DatawriterCryptoHandle,
    sending_datareader_crypto: DatareaderCryptoHandle,
  ) -> SecurityResult<Submessage> {
    todo!();
  }

  fn decode_serialized_payload(
    encoded_buffer: CryptoContent,
    inline_qos: ParameterList,
    receiving_datareader_crypto: DatareaderCryptoHandle,
    sending_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<SerializedPayload> {
    todo!();
  }
}
