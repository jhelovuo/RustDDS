use crate::{
  security::{
    access_control::{
      access_control_builtin::types::{
        BuiltinPluginEndpointSecurityAttributes, BuiltinPluginParticipantSecurityAttributes,
      },
      types::*,
    },
    cryptographic::cryptographic_builtin::*,
  },
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

  fn is_volatile_(properties: &[Property]) -> bool {
    properties
      .iter()
      .find(|property| property.name.eq("dds.sec.builtin_endpoint_name"))
      .map_or(false, |property| {
        property
          .value
          .eq("BuiltinParticipantVolatileMessageSecureWriter")
          || property
            .value
            .eq("BuiltinParticipantVolatileMessageSecureReader")
      })
  }

  fn use_256_bit_key_(properties: &[Property]) -> bool {
    properties
      .iter()
      .find(|property| property.name.eq("dds.sec.crypto.keysize"))
      .map_or(true, |property| !property.value.eq("128"))
  }

  fn transformation_kind_(
    is_protected: bool,
    is_encrypted: bool,
    use_256_bit_key: bool,
  ) -> BuiltinCryptoTransformationKind {
    match (is_protected, is_encrypted, use_256_bit_key) {
      (false, _, _) => BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE,
      (true, false, false) => {
        BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      }
      (true, false, true) => {
        BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC
      }
      (true, true, false) => BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM,
      (true, true, true) => BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM,
    }
  }

  //TODO replace with proper functionality
  fn generate_key_material_(
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
    Self::generate_key_material_(
      handle,
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE,
    )
  }

  fn generate_receiver_specific_key_(
    &mut self,
    keymat: KeyMaterial_AES_GCM_GMAC_seq,
    origin_authentication: bool,
    handle: CryptoHandle,
  ) -> KeyMaterial_AES_GCM_GMAC_seq {
    if origin_authentication {
      let master_receiver_specific_key = keygen(keymat.key().transformation_kind.into());
      keymat.add_receiver_specific_key(handle, master_receiver_specific_key)
    } else {
      keymat.add_receiver_specific_key(0, BuiltinKey::new())
    }
  }

  fn unregister_entity_(&mut self, entity_info: EntityInfo) {
    let entity_handle = entity_info.handle;
    self.encode_keys_.remove(&entity_handle);
    self.decode_keys_.remove(&entity_handle);
    self.entity_encrypt_options_.remove(&entity_handle);
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
    participant_properties: &[Property],
    participant_security_attributes: ParticipantSecurityAttributes,
  ) -> SecurityResult<ParticipantCryptoHandle> {
    //TODO: this is only a mock implementation
    let plugin_participant_security_attributes =
      BuiltinPluginParticipantSecurityAttributes::try_from(
        participant_security_attributes.plugin_participant_attributes,
      )?;
    let handle = self.generate_handle_();

    let key_material = Self::generate_key_material_(
      handle,
      Self::transformation_kind_(
        participant_security_attributes.is_rtps_protected,
        plugin_participant_security_attributes.is_rtps_encrypted,
        Self::use_256_bit_key_(participant_properties),
      ),
    );
    self
      .insert_encode_keys_(handle, KeyMaterial_AES_GCM_GMAC_seq::One(key_material))
      .and(self.insert_participant_attributes_(handle, participant_security_attributes))
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

    let is_rtps_origin_authenticated = self
      .participant_encrypt_options_
      .get(&local_participant_crypto_handle)
      .ok_or_else(|| {
        security_error!(
          "Participant encrypt options not found for the handle {}",
          local_participant_crypto_handle
        )
      })
      .and_then(|participant_security_attributes| {
        BuiltinPluginParticipantSecurityAttributes::try_from(
          participant_security_attributes.plugin_participant_attributes,
        )
      })
      .map(|plugin_participant_attributes| {
        plugin_participant_attributes.is_rtps_origin_authenticated
      })?;

    let remote_participant_handle = self.generate_handle_();

    let keys = self.generate_receiver_specific_key_(
      local_participant_keys,
      is_rtps_origin_authenticated,
      remote_participant_handle,
    );

    self.insert_encode_keys_(remote_participant_handle, keys)?;

    Ok(remote_participant_handle)
  }

  fn register_local_datawriter(
    &mut self,
    participant_crypto: ParticipantCryptoHandle,
    datawriter_properties: &[Property],
    datawriter_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatawriterCryptoHandle> {
    //TODO: this is only a mock implementation
    let plugin_endpoint_security_attributes = BuiltinPluginEndpointSecurityAttributes::try_from(
      datawriter_security_attributes.plugin_endpoint_attributes,
    )?;

    let local_datawriter_handle = self.generate_handle_();

    if Self::is_volatile_(datawriter_properties) {
      // By 8.8.8.3
      Err(security_error!(
        "register_local_datawriter should not be called 
        for BuiltinParticipantVolatileMessageSecureWriter"
      ))
    } else {
      let use_256_bit_key = Self::use_256_bit_key_(datawriter_properties);
      let submessage_transformation_kind = Self::transformation_kind_(
        datawriter_security_attributes.is_submessage_protected,
        plugin_endpoint_security_attributes.is_submessage_encrypted,
        use_256_bit_key,
      );
      let payload_transformation_kind = Self::transformation_kind_(
        datawriter_security_attributes.is_payload_protected,
        plugin_endpoint_security_attributes.is_payload_encrypted,
        use_256_bit_key,
      );

      let submessage_key_material =
        Self::generate_key_material_(local_datawriter_handle, submessage_transformation_kind);
      // If the transformation kinds match, key reuse is possible: 9.5.3.1
      let keys = if submessage_transformation_kind == payload_transformation_kind
      /* && additional configurable condition? */
      {
        KeyMaterial_AES_GCM_GMAC_seq::One(submessage_key_material)
      } else {
        KeyMaterial_AES_GCM_GMAC_seq::Two(
          submessage_key_material,
          Self::generate_key_material_(self.generate_handle_(), payload_transformation_kind),
        )
      };
      self.insert_encode_keys_(local_datawriter_handle, keys)?;

      self.insert_entity_attributes_(local_datawriter_handle, datawriter_security_attributes)?;

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
    let local_datawriter_key_materials: KeyMaterial_AES_GCM_GMAC_seq = self
      .get_encode_keys_(&local_datawriter_crypto_handle)
      .cloned()?;

    let is_submessage_origin_authenticated = self
      .entity_encrypt_options_
      .get(&local_datawriter_crypto_handle)
      .ok_or_else(|| {
        security_error!(
          "Datawriter encrypt options not found for the handle {}",
          local_datawriter_crypto_handle
        )
      })
      .and_then(|datawriter_security_attributes| {
        BuiltinPluginEndpointSecurityAttributes::try_from(
          datawriter_security_attributes.plugin_endpoint_attributes,
        )
      })
      .map(|plugin_datawriter_attributes| {
        plugin_datawriter_attributes.is_submessage_origin_authenticated
      })?;

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
      .entity_encrypt_options_
      .get(&local_datawriter_crypto_handle)
      .cloned()
    {
      self
        .entity_encrypt_options_
        .insert(remote_datareader_handle, attributes);
    }

    let keys = self.generate_receiver_specific_key_(
      local_datawriter_key_materials,
      is_submessage_origin_authenticated,
      remote_datareader_handle,
    );

    self.insert_encode_keys_(remote_datareader_handle, keys)?;

    Ok(remote_datareader_handle)
  }

  fn register_local_datareader(
    &mut self,
    participant_crypto: ParticipantCryptoHandle,
    datareader_properties: &[Property],
    datareader_security_attributes: EndpointSecurityAttributes,
  ) -> SecurityResult<DatareaderCryptoHandle> {
    //TODO: this is only a mock implementation
    let plugin_endpoint_security_attributes = BuiltinPluginEndpointSecurityAttributes::try_from(
      datareader_security_attributes.plugin_endpoint_attributes,
    )?;

    let local_datareader_handle = self.generate_handle_();
    if Self::is_volatile_(datareader_properties) {
      // By 8.8.8.3
      Err(security_error!(
        "register_local_datareader should not be called 
        for BuiltinParticipantVolatileMessageSecureReader"
      ))
    } else {
      // TODO check datareader_security_attributes.is_submessage_protected
      self.insert_encode_keys_(
        local_datareader_handle,
        KeyMaterial_AES_GCM_GMAC_seq::One(Self::generate_key_material_(
          local_datareader_handle,
          Self::transformation_kind_(
            datareader_security_attributes.is_submessage_protected,
            plugin_endpoint_security_attributes.is_submessage_encrypted,
            Self::use_256_bit_key_(datareader_properties),
          ),
        )),
      )?;

      self.insert_entity_attributes_(local_datareader_handle, datareader_security_attributes)?;

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
    let local_datareader_key_materials = self
      .get_encode_keys_(&local_datareader_crypto_handle)
      .cloned()?;

    let is_submessage_origin_authenticated = self
      .entity_encrypt_options_
      .get(&local_datareader_crypto_handle)
      .ok_or_else(|| {
        security_error!(
          "Datareader encrypt options not found for the handle {}",
          local_datareader_crypto_handle
        )
      })
      .and_then(|datareader_security_attributes| {
        BuiltinPluginEndpointSecurityAttributes::try_from(
          datareader_security_attributes.plugin_endpoint_attributes,
        )
      })
      .map(|plugin_datareader_attributes| {
        plugin_datareader_attributes.is_submessage_origin_authenticated
      })?;

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
      .entity_encrypt_options_
      .get(&local_datareader_crypto_handle)
      .cloned()
    {
      self
        .entity_encrypt_options_
        .insert(remote_datawriter_handle, attributes);
    }

    let keys = self.generate_receiver_specific_key_(
      local_datareader_key_materials,
      is_submessage_origin_authenticated,
      remote_datawriter_handle,
    );

    // Copy the keys
    self.insert_encode_keys_(remote_datawriter_handle, keys)?;

    Ok(remote_datawriter_handle)
  }

  fn unregister_participant(
    &mut self,
    participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    self
      .participant_encrypt_options_
      .remove(&participant_crypto_handle);
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
