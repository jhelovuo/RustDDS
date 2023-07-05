use std::collections::HashMap;

use speedy::Writable;

use crate::{
  messages::submessages::elements::{
    crypto_content::CryptoContent, crypto_header::CryptoHeader, parameter_list::ParameterList,
    serialized_payload::SerializedPayload,
  },
  messages::submessages::submessages::{
    ReaderSubmessage, WriterSubmessage,
  },
  messages::submessages::{
    secure_prefix::SecurePrefix,
    secure_postfix::SecurePostfix,
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
pub struct CryptographicBuiltIn {
  keys_: HashMap<CryptoHandle, Vec<KeyMaterial_AES_GCM_GMAC>>,
  encrypt_options_: HashMap<CryptoHandle, EndpointSecurityAttributes>,
  participant_to_entity_: HashMap<ParticipantCryptoHandle, Vec<EntityInfo>>,
  // sessions_ ?
  derived_key_handles_: HashMap<(ParticipantCryptoHandle, CryptoHandle), CryptoHandle>,

  handle_counter_: u32,
}

impl super::Cryptographic for CryptographicBuiltIn {}

impl CryptographicBuiltIn {
  pub fn new() -> Self {
    CryptographicBuiltIn {
      keys_: HashMap::new(),
      encrypt_options_: HashMap::new(),
      participant_to_entity_: HashMap::new(),
      derived_key_handles_: HashMap::new(),
      handle_counter_: 0,
    }
  }

  fn generate_handle_(&mut self) -> CryptoHandle {
    self.handle_counter_ += 1;
    self.handle_counter_
  }

  fn is_volatile_(properties: Vec<Property>) -> bool {
    match properties
      .into_iter()
      .find(|property| property.clone().name.eq("dds.sec.builtin_endpoint_name"))
    {
      Some(property) => {
        property
          .value
          .eq("BuiltinParticipantVolatileMessageSecureWriter")
          || property
            .value
            .eq("BuiltinParticipantVolatileMessageSecureReader")
      }
      None => false,
    }
  }

  //TODO replace with proper functionality
  fn generate_mock_key_(handle: CryptoHandle) -> KeyMaterial_AES_GCM_GMAC {
    KeyMaterial_AES_GCM_GMAC {
      transformation_kind: BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE,
      master_salt: Vec::new(),
      sender_key_id: handle,
      master_sender_key: Vec::new(),
      receiver_specific_key_id: 0,
      master_receiver_specific_key: Vec::new(),
    }
  }

  fn insert_keys_(
    &mut self,
    handle: CryptoHandle,
    keys: Vec<KeyMaterial_AES_GCM_GMAC>,
  ) -> SecurityResult<()> {
    match self.keys_.insert(handle, keys) {
      None => SecurityResult::Ok(()),
      Some(old_key_materials) => {
        self.keys_.insert(handle, old_key_materials);
        SecurityResult::Err(SecurityError {
          msg: format!(
            "The handle {} was already associated with key material",
            handle
          ),
        })
      }
    }
  }

  fn insert_attributes_(
    &mut self,
    handle: CryptoHandle,
    attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<()> {
    match self.encrypt_options_.insert(handle, attributes) {
      None => SecurityResult::Ok(()),
      Some(old_attributes) => {
        self.encrypt_options_.insert(handle, old_attributes);
        SecurityResult::Err(SecurityError {
          msg: format!(
            "The handle {} was already associated with security attributes",
            handle
          ),
        })
      }
    }
  }
}

/// Builtin CryptoKeyFactory implementation from section 9.5.3.1 of the Security
/// specification (v. 1.1)
impl CryptoKeyFactory for CryptographicBuiltIn {
  fn register_local_participant(
    &mut self,
    participant_identity: IdentityHandle,
    participant_permissions: PermissionsHandle,
    participant_properties: Vec<Property>,
    participant_security_attributes: ParticipantSecurityAttributes,
  ) -> SecurityResult<ParticipantCryptoHandle> {
    //TODO: this is only a mock implementation
    let handle = self.generate_handle_();
    let key_material = Self::generate_mock_key_(handle);
    self
      .insert_keys_(handle, vec![key_material])
      .map(|_| handle)
  }

  fn register_matched_remote_participant(
    &mut self,
    local_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_identity: IdentityHandle,
    remote_participant_permissions: PermissionsHandle,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<ParticipantCryptoHandle> {
    //TODO: this is only a mock implementation
    SecurityResult::Ok(self.generate_handle_())
  }

  fn register_local_datawriter(
    &mut self,
    participant_crypto: ParticipantCryptoHandle,
    datawriter_properties: Vec<Property>,
    datawriter_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatawriterCryptoHandle> {
    //TODO: this is only a mock implementation
    let handle = self.generate_handle_();
    let mut keys = Vec::<KeyMaterial_AES_GCM_GMAC>::new();
    if Self::is_volatile_(datawriter_properties) {
      todo!()
    } else {
      if false
      /* datawriter_security_attributes.is_submessage_protected */
      {
        keys.push(Self::generate_mock_key_(handle));
      }
      if false
      /* datawriter_security_attributes.is_payload_protected */
      {
        if keys.is_empty() {
          keys.push(Self::generate_mock_key_(handle));
        } else {
          keys.push(Self::generate_mock_key_(self.generate_handle_()));
        }
      }
      self.insert_keys_(handle, keys)?;
      self.insert_attributes_(handle, datawriter_security_attributes)?;
      if participant_crypto != 0 {
        let entity_info = EntityInfo {
          handle,
          category: EntityCategory::DataWriter,
        };
        match self.participant_to_entity_.get_mut(&participant_crypto) {
          Some(v) => v.push(entity_info),
          None => {
            self
              .participant_to_entity_
              .insert(participant_crypto, vec![entity_info]);
          }
        };
      }
      SecurityResult::Ok(handle)
    }
  }

  fn register_matched_remote_datareader(
    &mut self,
    local_datawriter_crypto_handle: DatawriterCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
    shared_secret: SharedSecretHandle,
    relay_only: bool,
  ) -> SecurityResult<DatareaderCryptoHandle> {
    //TODO: this is only a mock implementation
    let keys = self
      .keys_
      .get(&local_datawriter_crypto_handle)
      .ok_or_else(|| SecurityError {
        msg: format!(
          "No keys found for local_datawriter_crypto_handle {}",
          local_datawriter_crypto_handle
        ),
      })?;

    // Find a handle for the remote datareader corresponding to the (remote
    // participant, local datawriter) pair, or generate a new one
    let handle = self
      .derived_key_handles_
      .get(&(remote_participant_crypto, local_datawriter_crypto_handle))
      .copied()
      .unwrap_or(self.generate_handle_());

    if false
    /* use derived key */
    {
      todo!();
    }

    // Add entity info
    if remote_participant_crypto != 0 {
      let entity_info = EntityInfo {
        handle,
        category: EntityCategory::DataReader,
      };
      match self
        .participant_to_entity_
        .get_mut(&remote_participant_crypto)
      {
        Some(v) => v.push(entity_info),
        None => {
          self
            .participant_to_entity_
            .insert(remote_participant_crypto, vec![entity_info]);
        }
      };
    }

    // Copy the attributes
    if let Some(attributes) = self
      .encrypt_options_
      .get(&local_datawriter_crypto_handle)
      .copied()
    {
      self.encrypt_options_.insert(handle, attributes);
    }

    SecurityResult::Ok(handle)
  }

  fn register_local_datareader(
    &mut self,
    participant_crypto: ParticipantCryptoHandle,
    datareader_properties: Vec<Property>,
    datareader_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatareaderCryptoHandle> {
    //TODO: this is only a mock implementation
    let handle = self.generate_handle_();
    let mut keys = Vec::<KeyMaterial_AES_GCM_GMAC>::new();
    if Self::is_volatile_(datareader_properties) {
      // By 8.8.8.3
      SecurityResult::Err(SecurityError { msg: String::from("register_local_datareader should not be called for BuiltinParticipantVolatileMessageSecureReader") })
    } else {
      if false
      /* datareader_security_attributes.is_submessage_protected */
      {
        keys.push(Self::generate_mock_key_(handle));
      }
      if false
      /* datareader_security_attributes.is_payload_protected */
      {
        if keys.is_empty() {
          keys.push(Self::generate_mock_key_(handle));
        } else {
          keys.push(Self::generate_mock_key_(self.generate_handle_()));
        }
      }
      self.insert_keys_(handle, keys)?;
      self.insert_attributes_(handle, datareader_security_attributes)?;
      if participant_crypto != 0 {
        let entity_info = EntityInfo {
          handle,
          category: EntityCategory::DataReader,
        };
        match self.participant_to_entity_.get_mut(&participant_crypto) {
          Some(v) => v.push(entity_info),
          None => {
            self
              .participant_to_entity_
              .insert(participant_crypto, vec![entity_info]);
          }
        };
      }
      SecurityResult::Ok(handle)
    }
  }

  fn register_matched_remote_datawriter(
    &mut self,
    local_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<DatawriterCryptoHandle> {
    //TODO: this is only a mock implementation
    let keys = self
      .keys_
      .get(&local_datareader_crypto_handle)
      .ok_or_else(|| SecurityError {
        msg: format!(
          "No keys found for local_datareader_crypto_handle {}",
          local_datareader_crypto_handle
        ),
      })?;

    // Find a handle for the remote datawriter corresponding to the (remote
    // participant, local datareader) pair, or generate a new one
    let handle: DatareaderCryptoHandle = self
      .derived_key_handles_
      .get(&(remote_participant_crypto, local_datareader_crypto_handle))
      .copied()
      .unwrap_or(self.generate_handle_());

    if false
    /* use derived key */
    {
      todo!();
    }

    // Add entity info
    if remote_participant_crypto != 0 {
      let entity_info = EntityInfo {
        handle,
        category: EntityCategory::DataWriter,
      };
      match self
        .participant_to_entity_
        .get_mut(&remote_participant_crypto)
      {
        Some(v) => v.push(entity_info),
        None => {
          self
            .participant_to_entity_
            .insert(remote_participant_crypto, vec![entity_info]);
        }
      };
    }

    // Copy the attributes
    if let Some(attributes) = self
      .encrypt_options_
      .get(&local_datareader_crypto_handle)
      .copied()
    {
      self.encrypt_options_.insert(handle, attributes);
    }

    SecurityResult::Ok(handle)
  }

  fn unregister_participant(
    &mut self,
    participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }

  fn unregister_datawriter(
    &mut self,
    datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }

  fn unregister_datareader(
    &mut self,
    datareader_crypto_handle: DatareaderCryptoHandle,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }
}

impl CryptoKeyExchange for CryptographicBuiltIn {
  fn create_local_participant_crypto_tokens(
    &mut self,
    local_participant_crypto: ParticipantCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
  ) -> SecurityResult<Vec<ParticipantCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)
    self
      .keys_
      .get(&local_participant_crypto)
      .ok_or(SecurityError {
        msg: format!(
          "Could not find keys for the handle {}",
          local_participant_crypto
        ),
      })
      .cloned()
      // Convert to CryptoTokens
      .and_then(|keys| Vec::<DatawriterCryptoToken>::try_from(KeyMaterial_AES_GCM_GMAC_seq(keys)))
  }

  fn set_remote_participant_crypto_tokens(
    &mut self,
    local_participant_crypto: ParticipantCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
    remote_participant_tokens: Vec<ParticipantCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation (or is it?)
    KeyMaterial_AES_GCM_GMAC_seq::try_from(remote_participant_tokens).and_then(
      |KeyMaterial_AES_GCM_GMAC_seq(keymat_seq)| {
        self.insert_keys_(remote_participant_crypto, keymat_seq)
      },
    )
  }

  fn create_local_datawriter_crypto_tokens(
    &mut self,
    local_datawriter_crypto: DatawriterCryptoHandle,
    remote_datareader_crypto: DatareaderCryptoHandle,
  ) -> SecurityResult<Vec<DatawriterCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)

    self
      .keys_
      .get(&local_datawriter_crypto)
      .ok_or(SecurityError {
        msg: format!(
          "Could not find keys for the handle {}",
          local_datawriter_crypto
        ),
      })
      .cloned()
      // Convert to CryptoTokens
      .and_then(|keys| Vec::<DatawriterCryptoToken>::try_from(KeyMaterial_AES_GCM_GMAC_seq(keys)))
  }

  fn set_remote_datawriter_crypto_tokens(
    &mut self,
    local_datareader_crypto: DatareaderCryptoHandle,
    remote_datawriter_crypto: DatawriterCryptoHandle,
    remote_datawriter_tokens: Vec<DatawriterCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    KeyMaterial_AES_GCM_GMAC_seq::try_from(remote_datawriter_tokens).and_then(
      |KeyMaterial_AES_GCM_GMAC_seq(keymat_seq)| {
        self.insert_keys_(remote_datawriter_crypto, keymat_seq)
      },
    )
  }

  fn create_local_datareader_crypto_tokens(
    &mut self,
    local_datareader_crypto: DatareaderCryptoHandle,
    remote_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<Vec<DatareaderCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)

    self
      .keys_
      .get(&local_datareader_crypto)
      .ok_or(SecurityError {
        msg: format!(
          "Could not find keys for the handle {}",
          local_datareader_crypto
        ),
      })
      .cloned()
      // Convert to CryptoTokens
      .and_then(|keys| Vec::<DatawriterCryptoToken>::try_from(KeyMaterial_AES_GCM_GMAC_seq(keys)))
  }

  fn set_remote_datareader_crypto_tokens(
    &mut self,
    local_datawriter_crypto: DatawriterCryptoHandle,
    remote_datareader_crypto: DatareaderCryptoHandle,
    remote_datareader_tokens: Vec<DatareaderCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    KeyMaterial_AES_GCM_GMAC_seq::try_from(remote_datareader_tokens).and_then(
      |KeyMaterial_AES_GCM_GMAC_seq(keymat_seq)| {
        self.insert_keys_(remote_datareader_crypto, keymat_seq)
      },
    )
  }

  fn return_crypto_tokens(&mut self, crypto_tokens: Vec<CryptoToken>) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }
}

impl CryptoTransform for CryptographicBuiltIn {
  fn encode_serialized_payload(
    &mut self,
    plain_buffer: SerializedPayload,
    sending_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<(CryptoContent, ParameterList)> {
    //TODO: this is only a mock implementation
    let plaintext = plain_buffer.write_to_vec().map_err(|err| SecurityError {
      msg: format!("Error converting SerializedPayload to byte vector: {}", err),
    })?;
    let keymat_seq = self
      .keys_
      .get(&sending_datawriter_crypto)
      .ok_or(SecurityError {
        msg: format!(
          "Could not find keys for the handle {}",
          sending_datawriter_crypto
        ),
      })
      .cloned()?;

    match keymat_seq.as_slice() {
      [keymat_0] => {
        let header = BuiltinCryptoHeader {
          transform_identifier: BuiltinCryptoTransformIdentifier {
            transformation_kind: keymat_0.transformation_kind,
            transformation_key_id: 0,
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
    &mut self,
    plain_rtps_submessage: Submessage,
    sending_datawriter_crypto: DatawriterCryptoHandle,
    receiving_datareader_crypto_list: Vec<DatareaderCryptoHandle>,
  ) -> SecurityResult<EncodeResult<EncodedSubmessage>> {
    //TODO: this is only a mock implementation

    let keymat_seq = self
      .keys_
      .get(&sending_datawriter_crypto)
      .ok_or(SecurityError {
        msg: format!(
          "Could not find keys for the handle {}",
          sending_datawriter_crypto
        ),
      })
      .cloned()?;

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
    &mut self,
    plain_rtps_submessage: Submessage,
    sending_datareader_crypto: DatareaderCryptoHandle,
    receiving_datawriter_crypto_list: Vec<DatawriterCryptoHandle>,
  ) -> SecurityResult<EncodeResult<EncodedSubmessage>> {
    //TODO: this is only a mock implementation

    let keymat_seq = self
      .keys_
      .get(&sending_datareader_crypto)
      .ok_or(SecurityError {
        msg: format!(
          "Could not find keys for the handle {}",
          sending_datareader_crypto
        ),
      })?;
    let keymat = keymat_seq
      .first()
      .ok_or(SecurityError {
        msg: format!(
          "Could not find keys for the handle {}",
          sending_datareader_crypto
        ),
      })
      .cloned()?;

    match keymat.transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => Ok(EncodeResult::One(
        EncodedSubmessage::Unencoded(plain_rtps_submessage),
      )),
      _ => todo!(),
    }
  }

  fn encode_rtps_message(
    &mut self,
    plain_rtps_message: Message,
    sending_participant_crypto: ParticipantCryptoHandle,
    receiving_participant_crypto_list: Vec<ParticipantCryptoHandle>,
  ) -> SecurityResult<EncodeResult<Message>> {
    //TODO: this is only a mock implementation

    let keymat_seq = self
      .keys_
      .get(&sending_participant_crypto)
      .ok_or(SecurityError {
        msg: format!(
          "Could not find keys for the handle {}",
          sending_participant_crypto
        ),
      })?;
    let keymat = keymat_seq
      .first()
      .ok_or(SecurityError {
        msg: format!(
          "Could not find keys for the handle {}",
          sending_participant_crypto
        ),
      })
      .cloned()?;

    match keymat.transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        Ok(EncodeResult::One(plain_rtps_message))
      }
      _ => todo!(),
    }
  }

  fn decode_rtps_message(
    &mut self,
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
    &mut self,
    encoded_rtps_submessage: &Submessage,
    receiving_participant_crypto: ParticipantCryptoHandle,
    sending_participant_crypto: ParticipantCryptoHandle,
  ) -> SecurityResult<SecureSubmessageCategory> {
    todo!(); // 9.5.3.3.5  Compare key ID to figure out which is which?
  }

  fn decode_datawriter_submessage(
    &mut self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    receiving_datareader_crypto: DatareaderCryptoHandle,
    sending_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<WriterSubmessage> {
    todo!();
  }

  fn decode_datareader_submessage(
    &mut self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    receiving_datawriter_crypto: DatawriterCryptoHandle,
    sending_datareader_crypto: DatareaderCryptoHandle,
  ) -> SecurityResult<ReaderSubmessage> {
    todo!();
  }

  fn decode_serialized_payload(
    &mut self,
    encoded_buffer: CryptoContent,
    inline_qos: ParameterList,
    receiving_datareader_crypto: DatareaderCryptoHandle,
    sending_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<SerializedPayload> {
    todo!();
  }
}
