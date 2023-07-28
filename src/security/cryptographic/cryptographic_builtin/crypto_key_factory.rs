use crate::{
  security::{access_control::types::*, cryptographic::cryptographic_builtin::*},
  security_error,
};
use super::aes_gcm_gmac::keygen;

impl CryptographicBuiltIn {
  fn generate_handle_(&mut self) -> CryptoHandle {
    self.handle_counter_ += 1;
    self.handle_counter_
  }

  fn get_or_generate_matched_remote_entity_handle_(
    &mut self,
    remote_participant_handle: ParticipantCryptoHandle,
    local_entity_handle: EntityCryptoHandle,
  ) -> EntityCryptoHandle {
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
  fn generate_key_(
    handle: CryptoHandle,
    transformation_kind: BuiltinCryptoTransformationKind,
  ) -> KeyMaterial_AES_GCM_GMAC {
    KeyMaterial_AES_GCM_GMAC {
      transformation_kind,
      // TODO
      master_salt: Vec::new(),

      sender_key_id: handle,
      master_sender_key: keygen(transformation_kind.into()),
      // Leave receiver-specific key empty initially
      receiver_specific_key_id: 0,
      master_receiver_specific_key: BuiltinKey::new(),
    }
  }

  fn generate_mock_key_(handle: CryptoHandle) -> KeyMaterial_AES_GCM_GMAC {
    Self::generate_key_(
      handle,
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE,
    )
  }

  fn generate_receiver_specific_key_(
    &mut self,
    keymat: KeyMaterial_AES_GCM_GMAC_seq,
    origin_authentication: bool,
  ) -> KeyMaterial_AES_GCM_GMAC_seq {
    if origin_authentication {
      let master_receiver_specific_key = keygen(keymat.key().transformation_kind.into());
      keymat.add_receiver_specific_key(self.generate_handle_(), master_receiver_specific_key)
    } else {
      keymat.add_receiver_specific_key(0, BuiltinKey::new())
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
      .insert_encode_keys_(handle, KeyMaterial_AES_GCM_GMAC_seq::One(key_material))
      .and(Ok(handle))
  }

  fn register_matched_remote_participant(
    &mut self,
    local_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_identity: IdentityHandle,
    remote_participant_permissions: PermissionsHandle,
    shared_secret: SharedSecretHandle,
  ) -> SecurityResult<ParticipantCryptoHandle> {
    //TODO: this is only a mock implementation

    let local_participant_keys = self
      .get_encode_keys_(&local_participant_crypto_handle)
      .cloned()?;

    let remote_participant_handle = self.generate_handle_();

    // TODO check Metadata Protection Kind 9.5.3.1 for origin authentication
    let origin_authentication = true;
    let keys = self.generate_receiver_specific_key_(local_participant_keys, origin_authentication);

    // Copy the keys
    self.insert_encode_keys_(remote_participant_handle, keys)?;

    Ok(remote_participant_handle)
  }

  fn register_local_datawriter(
    &mut self,
    participant_crypto: ParticipantCryptoHandle,
    datawriter_properties: Vec<Property>,
    datawriter_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatawriterCryptoHandle> {
    //TODO: this is only a mock implementation
    let local_datawriter_handle = self.generate_handle_();
    let keys = Vec::<KeyMaterial_AES_GCM_GMAC>::new();
    if Self::is_volatile_(datawriter_properties) {
      todo!()
    } else {
      // TODO check datawriter_security_attributes.is_submessage_protected and
      // datawriter_security_attributes.is_payload_protected
      let keys = KeyMaterial_AES_GCM_GMAC_seq::Two(
        Self::generate_mock_key_(local_datawriter_handle),
        Self::generate_mock_key_(self.generate_handle_()),
      );
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
    let keys = self
      .get_encode_keys_(&local_datawriter_crypto_handle)
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
      .cloned()
    {
      self
        .encrypt_options_
        .insert(remote_datareader_handle, attributes);
    }

    // TODO check Metadata Protection Kind 9.5.3.1 for origin authentication
    let origin_authentication = true;
    let keys = self.generate_receiver_specific_key_(keys, origin_authentication);

    // Copy the keys
    self.insert_encode_keys_(remote_datareader_handle, keys)?;

    Ok(remote_datareader_handle)
  }

  fn register_local_datareader(
    &mut self,
    participant_crypto: ParticipantCryptoHandle,
    datareader_properties: Vec<Property>,
    datareader_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatareaderCryptoHandle> {
    //TODO: this is only a mock implementation
    let local_datareader_handle = self.generate_handle_();
    let keys = Vec::<KeyMaterial_AES_GCM_GMAC>::new();
    if Self::is_volatile_(datareader_properties) {
      // By 8.8.8.3
      SecurityResult::Err(security_error!(
        "register_local_datareader should not be called 
        for BuiltinParticipantVolatileMessageSecureReader"
      ))
    } else {
      // TODO check datareader_security_attributes.is_submessage_protected
      self.insert_encode_keys_(
        local_datareader_handle,
        KeyMaterial_AES_GCM_GMAC_seq::One(Self::generate_mock_key_(local_datareader_handle)),
      )?;
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
    let keys = self
      .get_encode_keys_(&local_datareader_crypto_handle)
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
      .cloned()
    {
      self
        .encrypt_options_
        .insert(remote_datawriter_handle, attributes);
    }

    // TODO check Metadata Protection Kind 9.5.3.1 for origin authentication
    let origin_authentication = true;
    let keys = self.generate_receiver_specific_key_(keys, origin_authentication);

    // Copy the keys
    self.insert_encode_keys_(remote_datawriter_handle, keys)?;

    Ok(remote_datawriter_handle)
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
