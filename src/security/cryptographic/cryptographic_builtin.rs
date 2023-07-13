use std::collections::{HashMap, HashSet};

use speedy::Writable;

use crate::{
  messages::submessages::{
    elements::{
      crypto_content::CryptoContent, crypto_header::CryptoHeader, parameter_list::ParameterList,
      serialized_payload::SerializedPayload,
    },
    secure_postfix::SecurePostfix,
    secure_prefix::SecurePrefix,
    submessage::SecuritySubmessage,
    submessages::{ReaderSubmessage, WriterSubmessage},
  },
  rtps::{Message, Submessage, SubmessageBody},
  security::{
    access_control::types::*,
    authentication::types::*,
    cryptographic::{builtin_types::*, cryptographic_plugin::*, types::*},
    types::*,
  },
  security_error,
};

// A struct implementing the built-in Cryptographic plugin
// See sections 8.5 and 9.5 of the Security specification (v. 1.1)
pub struct CryptographicBuiltIn {
  encode_keys_: HashMap<CryptoHandle, Vec<KeyMaterial_AES_GCM_GMAC>>,
  decode_keys_: HashMap<CryptoHandle, Vec<KeyMaterial_AES_GCM_GMAC>>,
  encrypt_options_: HashMap<CryptoHandle, EndpointSecurityAttributes>,
  participant_to_entity_info_: HashMap<ParticipantCryptoHandle, HashSet<EntityInfo>>,
  // For reverse lookups
  entity_to_participant_: HashMap<CryptoHandle, ParticipantCryptoHandle>,

  // sessions_ ?
  /// For each (local datawriter (/datareader), remote participant) pair, stores
  /// the matched remote datareader (/datawriter)
  matched_remote_entity_: HashMap<CryptoHandle, HashMap<ParticipantCryptoHandle, CryptoHandle>>,
  ///For reverse lookups,  for each remote datawriter (/datareader), stores the
  /// matched local datareader (/datawriter)
  matched_local_entity_: HashMap<CryptoHandle, CryptoHandle>,

  handle_counter_: u32,
}

impl super::Cryptographic for CryptographicBuiltIn {}

impl CryptographicBuiltIn {
  pub fn new() -> Self {
    CryptographicBuiltIn {
      encode_keys_: HashMap::new(),
      decode_keys_: HashMap::new(),
      encrypt_options_: HashMap::new(),
      participant_to_entity_info_: HashMap::new(),
      entity_to_participant_: HashMap::new(),
      matched_remote_entity_: HashMap::new(),
      matched_local_entity_: HashMap::new(),
      handle_counter_: 0,
    }
  }

  fn generate_handle_(&mut self) -> CryptoHandle {
    self.handle_counter_ += 1;
    self.handle_counter_
  }

  fn get_or_generate_matched_remote_entity_handle_(
    &mut self,
    remote_participant_handle: ParticipantCryptoHandle,
    local_entity_handle: CryptoHandle,
  ) -> CryptoHandle {
    // If a corresponding handle exists, get and return
    if let Some(remote_entity_handle) = self
      .matched_remote_entity_
      .get(&local_entity_handle)
      .and_then(|remote_participant_to_remote_entity| {
        remote_participant_to_remote_entity.get(&remote_participant_handle)
      })
    {
      *remote_entity_handle
    } else {
      // Otherwise generate a new handle
      let remote_entity_handle = self.generate_handle_();
      // Associate it with the remote participant
      self
        .entity_to_participant_
        .insert(remote_entity_handle, remote_participant_handle);
      // Associate it with the local entity
      self
        .matched_local_entity_
        .insert(remote_entity_handle, local_entity_handle);
      // Insert it to the HashMap corresponding to the local entity
      if let Some(remote_participant_to_remote_entity) =
        self.matched_remote_entity_.get_mut(&local_entity_handle)
      {
        remote_participant_to_remote_entity.insert(remote_participant_handle, remote_entity_handle);
      } else {
        // Create a new HashMap if one does not yet exist
        self.matched_remote_entity_.insert(
          local_entity_handle,
          HashMap::from([(remote_participant_handle, remote_entity_handle)]),
        );
      }
      // Return the generated handle
      remote_entity_handle
    }
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

  fn insert_encode_keys_(
    &mut self,
    handle: CryptoHandle,
    keys: Vec<KeyMaterial_AES_GCM_GMAC>,
  ) -> SecurityResult<()> {
    match self.encode_keys_.insert(handle, keys) {
      None => SecurityResult::Ok(()),
      Some(old_key_materials) => {
        self.encode_keys_.insert(handle, old_key_materials);
        SecurityResult::Err(security_error!(
          "The handle {} was already associated with encode key material",
          handle
        ))
      }
    }
  }
  fn insert_decode_keys_(
    &mut self,
    handle: CryptoHandle,
    keys: Vec<KeyMaterial_AES_GCM_GMAC>,
  ) -> SecurityResult<()> {
    match self.decode_keys_.insert(handle, keys) {
      None => SecurityResult::Ok(()),
      Some(old_key_materials) => {
        self.decode_keys_.insert(handle, old_key_materials);
        SecurityResult::Err(security_error!(
          "The handle {} was already associated with decode key material",
          handle
        ))
      }
    }
  }

  fn insert_entity_info_(
    &mut self,
    participant_handle: ParticipantCryptoHandle,
    entity_info: EntityInfo,
  ) {
    match self
      .participant_to_entity_info_
      .get_mut(&participant_handle)
    {
      Some(entity_set) => {
        entity_set.insert(entity_info);
      }
      None => {
        self
          .participant_to_entity_info_
          .insert(participant_handle, HashSet::from([entity_info]));
      }
    };
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
        SecurityResult::Err(security_error!(
          "The handle {} was already associated with security attributes",
          handle
        ))
      }
    }
  }

  fn unregister_entity_(&mut self, entity_info: EntityInfo) {
    let entity_handle = entity_info.handle;
    self.encode_keys_.remove(&entity_handle);
    self.decode_keys_.remove(&entity_handle);
    self.encrypt_options_.remove(&entity_handle);
    if let Some(participant_handle) = self.entity_to_participant_.remove(&entity_handle) {
      if let Some(entity_info_set) = self
        .participant_to_entity_info_
        .get_mut(&participant_handle)
      {
        entity_info_set.remove(&entity_info);
      }

      // If the entity is remote remove the association to the corresponding local
      // entity
      if let Some(matched_local_entity_handle) = self.matched_local_entity_.remove(&entity_handle) {
        if let Some(remote_participant_to_remote_entity) = self
          .matched_remote_entity_
          .get_mut(&matched_local_entity_handle)
        {
          remote_participant_to_remote_entity.remove(&participant_handle);
        }
      }
      // If the entity is local, unregister all associated remote entities as they serve no purpose
      // on their own. TODO: should we do this or just sever the association?
      else if let Some(remote_participant_to_remote_entity) =
        self.matched_remote_entity_.remove(&entity_handle)
      {
        for remote_entity in remote_participant_to_remote_entity.values() {
          self.unregister_entity_(EntityInfo {
            handle: *remote_entity,
            category: entity_info.category.opposite(),
          });
        }
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
      .insert_encode_keys_(handle, vec![key_material])
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

    if let Some(local_participant_keys) = self
      .encode_keys_
      .get(&local_participant_crypto_handle)
      .cloned()
    {
      let remote_participant_handle = self.generate_handle_();
      self
        .insert_encode_keys_(
          remote_participant_handle,
          local_participant_keys
            .iter()
            .map(|key| {
              let KeyMaterial_AES_GCM_GMAC {
                transformation_kind,
                master_salt,
                master_sender_key,
                sender_key_id,
                ..
              } = key.clone();
              let receiver_specific_key_id;
              let master_receiver_specific_key;
              // TODO check RTPS Protection Kind 9.5.3.1
              if true {
                receiver_specific_key_id = 0;
                master_receiver_specific_key = Vec::new();
              } else {
                // TODO create a receiver specific key
                receiver_specific_key_id = 0;
                master_receiver_specific_key = vec![0; 32];
              }
              KeyMaterial_AES_GCM_GMAC {
                transformation_kind,
                master_salt,
                master_sender_key,
                sender_key_id,
                receiver_specific_key_id,
                master_receiver_specific_key,
              }
            })
            .collect(),
        )
        .and(Ok(remote_participant_handle))
    } else {
      Err(security_error!(
        "Could not find encode keys for the local participant {}",
        local_participant_crypto_handle
      ))
    }
  }

  fn register_local_datawriter(
    &mut self,
    participant_crypto: ParticipantCryptoHandle,
    datawriter_properties: Vec<Property>,
    datawriter_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatawriterCryptoHandle> {
    //TODO: this is only a mock implementation
    let local_datawriter_handle = self.generate_handle_();
    let mut keys = Vec::<KeyMaterial_AES_GCM_GMAC>::new();
    if Self::is_volatile_(datawriter_properties) {
      todo!()
    } else {
      if false
      /* datawriter_security_attributes.is_submessage_protected */
      {
        keys.push(Self::generate_mock_key_(local_datawriter_handle));
      }
      if false
      /* datawriter_security_attributes.is_payload_protected */
      {
        if keys.is_empty() {
          keys.push(Self::generate_mock_key_(local_datawriter_handle));
        } else {
          keys.push(Self::generate_mock_key_(self.generate_handle_()));
        }
      }
      self.insert_encode_keys_(local_datawriter_handle, keys)?;
      self.insert_attributes_(local_datawriter_handle, datawriter_security_attributes)?;
      self.insert_entity_info_(
        participant_crypto,
        EntityInfo {
          handle: local_datawriter_handle,
          category: EntityCategory::DataWriter,
        },
      );

      self
        .entity_to_participant_
        .insert(local_datawriter_handle, participant_crypto);
      SecurityResult::Ok(local_datawriter_handle)
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
    let mut keys = self
      .encode_keys_
      .get(&local_datawriter_crypto_handle)
      .ok_or(security_error!(
        "No encode keys found for local_datawriter_crypto_handle {}",
        local_datawriter_crypto_handle
      ))
      .cloned()?;

    // Find a handle for the remote datareader corresponding to the (remote
    // participant, local datawriter) pair, or generate a new one
    let remote_datareader_handle = self.get_or_generate_matched_remote_entity_handle_(
      remote_participant_crypto,
      local_datawriter_crypto_handle,
    );

    if false
    /* use derived key */
    {
      todo!();
    }

    // Add entity info
    self.insert_entity_info_(
      remote_participant_crypto,
      EntityInfo {
        handle: remote_datareader_handle,
        category: EntityCategory::DataReader,
      },
    );

    // Copy the attributes
    if let Some(attributes) = self
      .encrypt_options_
      .get(&local_datawriter_crypto_handle)
      .copied()
    {
      self
        .encrypt_options_
        .insert(remote_datareader_handle, attributes);
    }

    // Copy the keys
    // TODO check RTPS Protection Kind 9.5.3.1
    if true {
      self.insert_encode_keys_(remote_datareader_handle, keys)?;
    } else {
      if let Some(KeyMaterial_AES_GCM_GMAC {
        transformation_kind,
        master_salt,
        master_sender_key,
        sender_key_id,
        ..
      }) = keys.first().cloned()
      {
        keys.insert(
          0,
          KeyMaterial_AES_GCM_GMAC {
            transformation_kind,
            master_salt,
            master_sender_key,
            sender_key_id,
            receiver_specific_key_id: 0,
            master_receiver_specific_key: Vec::new(),
          },
        );
      }
      self.insert_encode_keys_(remote_datareader_handle, keys)?;
    }

    SecurityResult::Ok(remote_datareader_handle)
  }

  fn register_local_datareader(
    &mut self,
    participant_crypto: ParticipantCryptoHandle,
    datareader_properties: Vec<Property>,
    datareader_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatareaderCryptoHandle> {
    //TODO: this is only a mock implementation
    let local_datareader_handle = self.generate_handle_();
    let mut keys = Vec::<KeyMaterial_AES_GCM_GMAC>::new();
    if Self::is_volatile_(datareader_properties) {
      // By 8.8.8.3
      SecurityResult::Err(security_error!("register_local_datareader should not be called for BuiltinParticipantVolatileMessageSecureReader") )
    } else {
      if false
      /* datareader_security_attributes.is_submessage_protected */
      {
        keys.push(Self::generate_mock_key_(local_datareader_handle));
      }
      if false
      /* datareader_security_attributes.is_payload_protected */
      {
        if keys.is_empty() {
          keys.push(Self::generate_mock_key_(local_datareader_handle));
        } else {
          keys.push(Self::generate_mock_key_(self.generate_handle_()));
        }
      }
      self.insert_encode_keys_(local_datareader_handle, keys)?;
      self.insert_attributes_(local_datareader_handle, datareader_security_attributes)?;
      self.insert_entity_info_(
        participant_crypto,
        EntityInfo {
          handle: local_datareader_handle,
          category: EntityCategory::DataReader,
        },
      );

      self
        .entity_to_participant_
        .insert(local_datareader_handle, participant_crypto);
      SecurityResult::Ok(local_datareader_handle)
    }
  }

  fn register_matched_remote_datawriter(
    &mut self,
    local_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<DatawriterCryptoHandle> {
    //TODO: this is only a mock implementation
    let mut keys = self
      .encode_keys_
      .get(&local_datareader_crypto_handle)
      .ok_or(security_error!(
        "No encode keys found for local_datareader_crypto_handle {}",
        local_datareader_crypto_handle
      ))
      .cloned()?;

    // Find a handle for the remote datawriter corresponding to the (remote
    // participant, local datareader) pair, or generate a new one
    let remote_datawriter_handle: DatareaderCryptoHandle = self
      .get_or_generate_matched_remote_entity_handle_(
        remote_participant_crypto,
        local_datareader_crypto_handle,
      );

    if false
    /* use derived key */
    {
      todo!();
    }

    // Add entity info
    self.insert_entity_info_(
      remote_participant_crypto,
      EntityInfo {
        handle: remote_datawriter_handle,
        category: EntityCategory::DataWriter,
      },
    );

    // Copy the attributes
    if let Some(attributes) = self
      .encrypt_options_
      .get(&local_datareader_crypto_handle)
      .copied()
    {
      self
        .encrypt_options_
        .insert(remote_datawriter_handle, attributes);
    }

    // Copy the keys
    // TODO check RTPS Protection Kind 9.5.3.1
    if true {
      self.insert_encode_keys_(remote_datawriter_handle, keys)?;
    } else {
      if let Some(KeyMaterial_AES_GCM_GMAC {
        transformation_kind,
        master_salt,
        master_sender_key,
        sender_key_id,
        ..
      }) = keys.first().cloned()
      {
        keys.insert(
          0,
          KeyMaterial_AES_GCM_GMAC {
            transformation_kind,
            master_salt,
            master_sender_key,
            sender_key_id,
            receiver_specific_key_id: 0,
            master_receiver_specific_key: Vec::new(),
          },
        );
      }
      self.insert_encode_keys_(remote_datawriter_handle, keys)?;
    }
    SecurityResult::Ok(remote_datawriter_handle)
  }

  fn unregister_participant(
    &mut self,
    participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    if let Some(entity_info_set) = self
      .participant_to_entity_info_
      .remove(&participant_crypto_handle)
    {
      for entity_info in entity_info_set {
        self.unregister_entity_(entity_info);
      }
    }
    Ok(())
  }

  fn unregister_datawriter(
    &mut self,
    datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    self.unregister_entity_(EntityInfo {
      handle: datawriter_crypto_handle,
      category: EntityCategory::DataWriter,
    });
    Ok(())
  }

  fn unregister_datareader(
    &mut self,
    datareader_crypto_handle: DatareaderCryptoHandle,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    self.unregister_entity_(EntityInfo {
      handle: datareader_crypto_handle,
      category: EntityCategory::DataReader,
    });
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
      .encode_keys_
      .get(&remote_participant_crypto)
      .ok_or(security_error!(
        "Could not find encode keys for the handle {}",
        remote_participant_crypto
      ))
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
        self.insert_decode_keys_(remote_participant_crypto, keymat_seq)
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
      .encode_keys_
      .get(&remote_datareader_crypto)
      .ok_or(security_error!(
        "Could not find encode keys for the handle {}",
        remote_datareader_crypto
      ))
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
        self.insert_decode_keys_(remote_datawriter_crypto, keymat_seq)
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
      .encode_keys_
      .get(&remote_datawriter_crypto)
      .ok_or(security_error!(
        "Could not find encode keys for the handle {}",
        remote_datawriter_crypto
      ))
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
        self.insert_decode_keys_(remote_datareader_crypto, keymat_seq)
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
    let plaintext = plain_buffer.write_to_vec().map_err(|err| {
      security_error!("Error converting SerializedPayload to byte vector: {}", err)
    })?;
    let keymat_seq = self
      .encode_keys_
      .get(&sending_datawriter_crypto)
      .ok_or(security_error!(
        "Could not find keys for the handle {}",
        sending_datawriter_crypto
      ))
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

            let header_vec = CryptoHeader::from(header).write_to_vec().map_err(|err| {
              security_error!("Error converting CryptoHeader to byte vector: {}", err)
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
      .encode_keys_
      .get(&sending_datawriter_crypto)
      .ok_or(security_error!(
        "Could not find keys for the handle {}",
        sending_datawriter_crypto
      ))
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

    let keymat = self
      .encode_keys_
      .get(&sending_datareader_crypto)
      .and_then(|v| v.first())
      .ok_or(security_error!(
        "Could not find keys for the handle {}",
        sending_datareader_crypto
      ))
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

    let keymat = self
      .encode_keys_
      .get(&sending_participant_crypto)
      .and_then(|v| v.first())
      .ok_or(security_error!(
        "Could not find keys for the handle {}",
        sending_participant_crypto
      ))
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
    // 9.5.3.3.5
    if let Submessage {
      body:
        SubmessageBody::Security(SecuritySubmessage::SecurePrefix(
          SecurePrefix {
            crypto_header:
              CryptoHeader {
                transformation_id:
                  CryptoTransformIdentifier {
                    transformation_kind,
                    transformation_key_id,
                  },
                ..
              },
            ..
          },
          _,
        )),
      ..
    } = *encoded_rtps_submessage
    {
      // Check the validity of transformation_kind
      let submessage_transformation_kind =
        BuiltinCryptoTransformationKind::try_from(transformation_kind)?;

      // Search for matching keys over entities registered to the sender
      let sending_participant_entities = self
        .participant_to_entity_info_
        .get(&sending_participant_crypto)
        .ok_or(security_error!(
          "Could not find registered entities for the sending_participant_crypto {}",
          sending_participant_crypto
        ))?;
      for EntityInfo { handle, category } in sending_participant_entities {
        // Iterate over the keys associated with the entity
        if let Some(keys) = self.decode_keys_.get(handle) {
          for KeyMaterial_AES_GCM_GMAC {
            transformation_kind,
            sender_key_id,
            ..
          } in keys
          {
            // Compare keys to the crypto transform identifier
            if *transformation_kind == submessage_transformation_kind
              && *sender_key_id == transformation_key_id
            {
              let remote_entity_handle = *handle;
              let matched_local_entity_handle = *self
                .matched_local_entity_
                .get(&remote_entity_handle)
                .ok_or(security_error!(
                  "The local entity matched to the remote entity handle {} is missing.",
                  remote_entity_handle
                ))?;
              return Ok(match category {
                EntityCategory::DataReader => SecureSubmessageCategory::DatareaderSubmessage(
                  remote_entity_handle,
                  matched_local_entity_handle,
                ),
                EntityCategory::DataWriter => SecureSubmessageCategory::DatawriterSubmessage(
                  remote_entity_handle,
                  matched_local_entity_handle,
                ),
              });
            }
          }
        }
      }
      // No matching keys were found for any entity registered to the sender
      Err( security_error!(
          "Could not find matching keys for any registered entity for the sending_participant_crypto {}.",
          sending_participant_crypto
        )
      )
    } else {
      Err(security_error!("preprocess_secure_submsg expects encoded_rtps_submessage to be a SEC_PREFIX. Received {:?}.",encoded_rtps_submessage.header.kind ) )
    }
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
