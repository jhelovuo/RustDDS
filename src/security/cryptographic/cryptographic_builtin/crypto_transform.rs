use enumflags2::BitFlags;
use speedy::{Readable, Writable};

use crate::{
  messages::submessages::{
    elements::{
      crypto_content::CryptoContent, crypto_footer::CryptoFooter, crypto_header::CryptoHeader,
      parameter_list::ParameterList,
    },
    info_source::InfoSource,
    secure_postfix::SecurePostfix,
    secure_prefix::SecurePrefix,
    secure_rtps_postfix::SecureRTPSPostfix,
    secure_rtps_prefix::SecureRTPSPrefix,
    submessage::SecuritySubmessage,
    submessage_flag::FromEndianness,
    submessages::{ReaderSubmessage, WriterSubmessage},
  },
  rtps::{Message, Submessage, SubmessageBody},
  security::cryptographic::cryptographic_builtin::{
    decode::{decode_rtps_message_gcm, decode_rtps_message_gmac},
    *,
  },
  security_error,
};
use super::{
  decode::{
    decode_datareader_submessage_gcm, decode_datareader_submessage_gmac,
    decode_datawriter_submessage_gcm, decode_datawriter_submessage_gmac,
    decode_serialized_payload_gcm, decode_serialized_payload_gmac, find_receiver_specific_mac,
  },
  encode::{
    encode_gcm, encode_gmac, encode_serialized_payload_gcm, encode_serialized_payload_gmac,
  },
  key_material::*,
};

impl CryptographicBuiltin {
  fn encode_submessage(
    &self,
    plain_rtps_submessage: Submessage,
    sending_endpoint_crypto_handle: EndpointCryptoHandle,
    receiving_endpoint_crypto_handle_list: &[EndpointCryptoHandle],
  ) -> SecurityResult<EncodedSubmessage> {
    //TODO: this is only a mock implementation

    // Serialize plaintext
    let plaintext = plain_rtps_submessage
      .write_to_vec()
      .map_err(|err| security_error!("Error converting Submessage to byte vector: {}", err))?;

    // Get the key material for encoding
    let sender_key_material = self
      .get_encode_key_materials(&sending_endpoint_crypto_handle)
      .map(KeyMaterial_AES_GCM_GMAC_seq::key_material)
      .cloned()?;

    // Get the key materials for computing receiver-specific MACs
    let receiver_specific_key_materials = SecurityResult::<Vec<ReceiverKeyMaterial>>::from_iter(
      // Iterate over receiver handles
      receiving_endpoint_crypto_handle_list
        .iter()
        .map(|receiver_crypto_handle| {
          // Get the encode key material
          self
            .get_encode_key_materials(receiver_crypto_handle)
            .map(KeyMaterial_AES_GCM_GMAC_seq::key_material)
            // Compare to the common key material and get the receiver specific key material
            .and_then(|receiver_key_material| {
              receiver_key_material.receiver_key_material_for(&sender_key_material)
            })
            // TODO use session keys
            .map(
              |ReceiverKeyMaterial {
                 receiver_specific_key_id,
                 master_receiver_specific_key,
               }| ReceiverKeyMaterial {
                receiver_specific_key_id,
                master_receiver_specific_key,
              },
            )
        }),
    )?;

    // Destructure the common key material
    let KeyMaterial_AES_GCM_GMAC {
      transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      ..
    } = sender_key_material;

    let initialization_vector = self.random_initialization_vector();

    // TODO use session keys
    let encode_key = &master_sender_key;

    // Compute encoded submessage and footer

    let (encoded_submessage, crypto_footer) = match transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        return Ok(EncodedSubmessage::Unencoded(plain_rtps_submessage))
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => (
        plain_rtps_submessage,
        encode_gmac(
          encode_key,
          initialization_vector,
          &plaintext,
          &receiver_specific_key_materials,
        )?,
      ),
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => encode_gcm(
        encode_key,
        initialization_vector,
        &plaintext,
        &receiver_specific_key_materials,
      )?,
    };

    // Build crypto header and security prefix
    let prefix = SecurePrefix {
      crypto_header: CryptoHeader::from(BuiltinCryptoHeader {
        transform_identifier: BuiltinCryptoTransformIdentifier {
          transformation_kind,
          transformation_key_id: sender_key_id,
        },
        builtin_crypto_header_extra: initialization_vector.into(),
      }),
    };

    // Build security postfix
    let postfix = SecurePostfix {
      crypto_footer: CryptoFooter::try_from(crypto_footer)?,
    };

    Ok(EncodedSubmessage::Encoded(
      prefix.create_submessage(speedy::Endianness::BigEndian)?, // 9.5.2.3 use BigEndian
      encoded_submessage,
      postfix.create_submessage(speedy::Endianness::BigEndian)?, // 9.5.2.5 use BigEndian
    ))
  }
}

impl CryptoTransform for CryptographicBuiltin {
  fn encode_serialized_payload(
    &self,
    plain_buffer: Vec<u8>,
    sending_datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<(Vec<u8>, ParameterList)> {
    // Get the key material for encrypting serialized payloads
    let payload_key_material = self
      .get_encode_key_materials(&sending_datawriter_crypto_handle)
      .map(KeyMaterial_AES_GCM_GMAC_seq::payload_key_material)?;

    let initialization_vector = self.random_initialization_vector();

    let header = BuiltinCryptoHeader {
      transform_identifier: BuiltinCryptoTransformIdentifier {
        transformation_kind: payload_key_material.transformation_kind,
        transformation_key_id: payload_key_material.sender_key_id,
      },
      builtin_crypto_header_extra: initialization_vector.into(),
    };

    // TODO use session key
    let encode_key = &payload_key_material.master_sender_key;

    let (encoded_data, footer) = match payload_key_material.transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        return Ok((plain_buffer, ParameterList::new()));
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
        encode_serialized_payload_gmac(encode_key, initialization_vector, &plain_buffer)?
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        encode_serialized_payload_gcm(encode_key, initialization_vector, &plain_buffer)?
      }
    };

    let header_vec = CryptoHeader::from(header)
      .write_to_vec()
      .map_err(|err| security_error!("Error converting CryptoHeader to byte vector: {}", err))?;
    let footer_vec = Vec::<u8>::try_from(footer)?;
    Ok((
      CryptoContent::from([header_vec, encoded_data, footer_vec].concat())
        .write_to_vec()
        .map_err(|e| security_error!("Error serializing CryptoContent: {e:?}"))?,
      ParameterList::new(),
    ))
  }

  fn encode_datawriter_submessage(
    &self,
    plain_rtps_submessage: Submessage,
    sending_datawriter_crypto_handle: DatawriterCryptoHandle,
    receiving_datareader_crypto_handle_list: Vec<DatareaderCryptoHandle>,
  ) -> SecurityResult<EncodedSubmessage> {
    self.encode_submessage(
      plain_rtps_submessage,
      sending_datawriter_crypto_handle,
      &receiving_datareader_crypto_handle_list,
    )
  }

  fn encode_datareader_submessage(
    &self,
    plain_rtps_submessage: Submessage,
    sending_datareader_crypto_handle: DatareaderCryptoHandle,
    receiving_datawriter_crypto_handle_list: Vec<DatawriterCryptoHandle>,
  ) -> SecurityResult<EncodedSubmessage> {
    self.encode_submessage(
      plain_rtps_submessage,
      sending_datareader_crypto_handle,
      &receiving_datawriter_crypto_handle_list,
    )
  }

  fn encode_rtps_message(
    &self,
    Message {
      header,
      submessages,
    }: Message,
    sending_participant_crypto_handle: ParticipantCryptoHandle,
    receiving_participant_crypto_handle_list: Vec<ParticipantCryptoHandle>,
  ) -> SecurityResult<Message> {
    // Convert the header into an InfoSource submessage
    let info_source = InfoSource::from(header)
      .create_submessage(BitFlags::from_endianness(speedy::Endianness::BigEndian));

    // Add info_source in front of the other submessages
    let submessages_with_info_source = [vec![info_source], submessages].concat();

    // Serialize plaintext
    let plaintext = SecurityResult::<Vec<Vec<u8>>>::from_iter(
      submessages_with_info_source
        // Serialize submessages
        .iter()
        .map(|submessage| {
          submessage
            .write_to_vec()
            .map_err(|err| security_error!("Error converting Submessage to byte vector: {}", err))
        }),
    )? // Deal with errors
    // Combine the serialized submessages
    .concat();

    // Get the key material for encoding
    let sender_key_material = self
      .get_encode_key_materials(&sending_participant_crypto_handle)
      .map(KeyMaterial_AES_GCM_GMAC_seq::key_material)
      .cloned()?;

    // Get the keys materials for computing receiver-specific MACs
    let receiver_specific_key_materials = SecurityResult::<Vec<ReceiverKeyMaterial>>::from_iter(
      // Iterate over receiver handles
      receiving_participant_crypto_handle_list
        .iter()
        .map(|receiver_handle| {
          // Get the encode key material
          self
            .get_encode_key_materials(receiver_handle)
            .map(KeyMaterial_AES_GCM_GMAC_seq::key_material)
            // Compare to the common key and get the receiver specific key
            .and_then(|receiver_key_material| {
              receiver_key_material.receiver_key_material_for(&sender_key_material)
            })
            // TODO use session keys
            .map(
              |ReceiverKeyMaterial {
                 receiver_specific_key_id,
                 master_receiver_specific_key,
               }| ReceiverKeyMaterial {
                receiver_specific_key_id,
                master_receiver_specific_key,
              },
            )
        }),
    )?;

    // Destructure the common key material
    let KeyMaterial_AES_GCM_GMAC {
      transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      ..
    } = sender_key_material;

    let initialization_vector = self.random_initialization_vector();

    // TODO use session keys
    let encode_key = &master_sender_key;

    // Compute encoded submessages and footer
    let (encoded_submessages, crypto_footer) = match transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        // TODO this is mainly for testing and debugging
        (
          submessages_with_info_source,
          encode_gmac(
            encode_key,
            initialization_vector,
            &plaintext,
            &receiver_specific_key_materials,
          )?,
        )
        /*  // TODO? switch to the following to avoid unnecessary pre/postfixes
        return Ok(EncodeResult::One(plain_rtps_message)); */
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => (
        submessages_with_info_source,
        encode_gmac(
          encode_key,
          initialization_vector,
          &plaintext,
          &receiver_specific_key_materials,
        )?,
      ),
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        encode_gcm(
          encode_key,
          initialization_vector,
          &plaintext,
          &receiver_specific_key_materials,
        )
        // Wrap the submessage in Vec
        .map(|(secure_body_submessage, footer)| (vec![secure_body_submessage], footer))?
      }
    };

    // Build crypto header and security prefix
    let prefix = SecurePrefix {
      crypto_header: CryptoHeader::from(BuiltinCryptoHeader {
        transform_identifier: BuiltinCryptoTransformIdentifier {
          transformation_kind,
          transformation_key_id: sender_key_id,
        },
        builtin_crypto_header_extra: initialization_vector.into(),
      }),
    };

    // Build security postfix
    let postfix = SecurePostfix {
      crypto_footer: CryptoFooter::try_from(crypto_footer)?,
    };

    Ok(Message {
      header,
      submessages: [
        vec![prefix.create_submessage(speedy::Endianness::BigEndian)?], // 9.5.2.3 use BigEndian
        encoded_submessages,
        vec![postfix.create_submessage(speedy::Endianness::BigEndian)?], // 9.5.2.5 use BigEndian
      ]
      .concat()
      .to_vec(),
    })
  }

  fn decode_rtps_message(
    &self,
    encoded_message: Message,
    _receiving_participant_crypto_handle: ParticipantCryptoHandle,
    sending_participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<Message> {
    //TODO: this is only a mock implementation

    let Message {
      header,
      submessages,
    } = encoded_message;

    if let [Submessage {
      body:
        SubmessageBody::Security(SecuritySubmessage::SecureRTPSPrefix(
          SecureRTPSPrefix { crypto_header, .. },
          _,
        )),
      ..
    }, encoded_content @ .., Submessage {
      body:
        SubmessageBody::Security(SecuritySubmessage::SecureRTPSPostfix(
          SecureRTPSPostfix { crypto_footer },
          _,
        )),
      ..
    }] = submessages.as_slice()
    {
      let BuiltinCryptoHeader {
        transform_identifier:
          BuiltinCryptoTransformIdentifier {
            transformation_kind: header_transformation_kind,
            transformation_key_id,
          },
        builtin_crypto_header_extra: BuiltinCryptoHeaderExtra(initialization_vector),
      } = BuiltinCryptoHeader::try_from(crypto_header.clone())?;

      let BuiltinCryptoFooter {
        common_mac,
        receiver_specific_macs,
      } = BuiltinCryptoFooter::try_from(crypto_footer.clone())?;

      // Get decode key material
      let KeyMaterial_AES_GCM_GMAC {
        transformation_kind: key_material_transformation_kind,
        master_salt,
        sender_key_id,
        master_sender_key,
        receiver_specific_key_id,
        master_receiver_specific_key,
      } = self
        .get_decode_key_materials(&sending_participant_crypto_handle)
        // Get the one for submessages (not only payload)
        .map(KeyMaterial_AES_GCM_GMAC_seq::key_material)?;

      // Check that the key id matches the header
      if !transformation_key_id.eq(sender_key_id) {
        Err(security_error!(
          "The key IDs don't match. The key material has sender_key_id {}, while the header has \
           transformation_key_id {}",
          sender_key_id,
          transformation_key_id
        ))?;
      } else if !header_transformation_kind.eq(key_material_transformation_kind) {
        Err(security_error!(
          "The transformation_kind don't match. The key material has {:?}, while the header has \
           {:?}",
          key_material_transformation_kind,
          header_transformation_kind
        ))?;
      }

      // Get the receiver-specific MAC if one is expected
      let receiver_specific_mac =
        find_receiver_specific_mac(*receiver_specific_key_id, &receiver_specific_macs)?;

      // TODO use session key?
      let decode_key = master_sender_key;
      // TODO use session key?
      let receiver_specific_key = master_receiver_specific_key;

      match key_material_transformation_kind {
        BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
          decode_rtps_message_gmac(
            decode_key,
            receiver_specific_key,
            initialization_vector,
            encoded_content,
            common_mac,
            receiver_specific_mac,
          )
        }
        BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
        | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
          decode_rtps_message_gmac(
            decode_key,
            receiver_specific_key,
            initialization_vector,
            encoded_content,
            common_mac,
            receiver_specific_mac,
          )
        }
        BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
        | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
          decode_rtps_message_gcm(
            decode_key,
            receiver_specific_key,
            initialization_vector,
            encoded_content,
            common_mac,
            receiver_specific_mac,
          )
        }
      }
      .and_then(|(submessages, info_source)| {
        if InfoSource::from(header).eq(&info_source) {
          Ok(Message {
            header,
            submessages,
          })
        } else {
          Err(security_error!(
            "The RTPS header did not match the MACed InfoSource: {:?} expected to match {:?}",
            info_source,
            header
          ))
        }
      })
    } else {
      Err(security_error!(
        "Expected the first submessage to be SecureRTPSPrefix and the last SecureRTPSPostfix"
      ))
    }
  }

  fn preprocess_secure_submessage(
    &self,
    secure_prefix: &SecurePrefix,
    _receiving_participant_crypto_handle: ParticipantCryptoHandle,
    sending_participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<SecureSubmessageKind> {
    // 9.5.3.3.5
    let SecurePrefix {
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
    } = *secure_prefix;

    // Check the validity of transformation_kind
    let submessage_transformation_kind =
      BuiltinCryptoTransformationKind::try_from(transformation_kind)?;

    // Search for matching key materials over endpoints registered to the sender
    let sending_participant_endpoints = self
      .participant_to_endpoint_info
      .get(&sending_participant_crypto_handle)
      .ok_or(security_error!(
        "Could not find registered entities for the sending_participant_crypto_handle {}",
        sending_participant_crypto_handle
      ))?;
    for EndpointInfo {
      crypto_handle: handle,
      kind: category,
    } in sending_participant_endpoints
    {
      // Iterate over the key materials associated with the endpoint
      if let Some(KeyMaterial_AES_GCM_GMAC {
        transformation_kind,
        sender_key_id,
        ..
      }) = self
        .decode_key_materials
        .get(handle)
        .map(KeyMaterial_AES_GCM_GMAC_seq::key_material)
      {
        // Compare key materials to the crypto transform identifier
        if submessage_transformation_kind.eq(transformation_kind)
          && transformation_key_id.eq(sender_key_id)
        {
          let remote_endpoint_crypto_handle = *handle;
          let matched_local_endpoint_crypto_handle = *self
            .matched_local_endpoint
            .get(&remote_endpoint_crypto_handle)
            .ok_or(security_error!(
              "The local endpoint matched to the remote endpoint crypto handle {} is missing.",
              remote_endpoint_crypto_handle
            ))?;
          return Ok(match category {
            EndpointKind::DataReader => SecureSubmessageKind::DatareaderSubmessage(
              remote_endpoint_crypto_handle,
              matched_local_endpoint_crypto_handle,
            ),
            EndpointKind::DataWriter => SecureSubmessageKind::DatawriterSubmessage(
              remote_endpoint_crypto_handle,
              matched_local_endpoint_crypto_handle,
            ),
          });
        }
      }
    }
    // No matching key materials were found for any endpoint registered to the
    // sender
    Err(security_error!(
      "Could not find matching key materials for any registered endpoint for the \
       sending_participant_crypto_handle {}.",
      sending_participant_crypto_handle
    ))
  }

  fn decode_datawriter_submessage(
    &self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    _receiving_datareader_crypto_handle: DatareaderCryptoHandle,
    sending_datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<WriterSubmessage> {
    //TODO: this is only a mock implementation

    // Destructure header and footer
    let (SecurePrefix { crypto_header }, encoded_submessage, SecurePostfix { crypto_footer }) =
      encoded_rtps_submessage;

    let BuiltinCryptoHeader {
      transform_identifier:
        BuiltinCryptoTransformIdentifier {
          transformation_kind: header_transformation_kind,
          transformation_key_id,
        },
      builtin_crypto_header_extra: BuiltinCryptoHeaderExtra(initialization_vector),
    } = BuiltinCryptoHeader::try_from(crypto_header)?;

    let BuiltinCryptoFooter {
      common_mac,
      receiver_specific_macs,
    } = BuiltinCryptoFooter::try_from(crypto_footer)?;

    // Get decode key material
    let KeyMaterial_AES_GCM_GMAC {
      transformation_kind: key_material_transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      receiver_specific_key_id,
      master_receiver_specific_key,
    } = self
      .get_decode_key_materials(&sending_datawriter_crypto_handle)
      // Get the one for submessages (not only payload)
      .map(KeyMaterial_AES_GCM_GMAC_seq::key_material)?;

    // Check that the key id matches the header. This should be redundant if the
    // method is called after preprocess_secure_submessage
    if !transformation_key_id.eq(sender_key_id) {
      Err(security_error!(
        "The key IDs don't match. The key material has sender_key_id {}, while the header has \
         transformation_key_id {}",
        sender_key_id,
        transformation_key_id
      ))?;
    } else if !header_transformation_kind.eq(key_material_transformation_kind) {
      Err(security_error!(
        "The transformation_kind don't match. The key material has {:?}, while the header has {:?}",
        key_material_transformation_kind,
        header_transformation_kind
      ))?;
    }

    // Get the receiver-specific MAC if one is expected
    let receiver_specific_mac =
      find_receiver_specific_mac(*receiver_specific_key_id, &receiver_specific_macs)?;

    // TODO use session key?
    let decode_key = master_sender_key;
    // TODO use session key?
    let receiver_specific_key = master_receiver_specific_key;

    match key_material_transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        decode_datawriter_submessage_gmac(
          decode_key,
          receiver_specific_key,
          initialization_vector,
          encoded_submessage,
          common_mac,
          receiver_specific_mac,
        )
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
        decode_datawriter_submessage_gmac(
          decode_key,
          receiver_specific_key,
          initialization_vector,
          encoded_submessage,
          common_mac,
          receiver_specific_mac,
        )
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        decode_datawriter_submessage_gcm(
          decode_key,
          receiver_specific_key,
          initialization_vector,
          encoded_submessage,
          common_mac,
          receiver_specific_mac,
        )
      }
    }
  }

  fn decode_datareader_submessage(
    &self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    _receiving_datawriter_crypto_handle: DatawriterCryptoHandle,
    sending_datareader_crypto_handle: DatareaderCryptoHandle,
  ) -> SecurityResult<ReaderSubmessage> {
    //TODO: this is only a mock implementation

    // Destructure header and footer
    let (SecurePrefix { crypto_header }, encoded_submessage, SecurePostfix { crypto_footer }) =
      encoded_rtps_submessage;

    let BuiltinCryptoHeader {
      transform_identifier:
        BuiltinCryptoTransformIdentifier {
          transformation_kind: header_transformation_kind,
          transformation_key_id,
        },
      builtin_crypto_header_extra: BuiltinCryptoHeaderExtra(initialization_vector),
    } = BuiltinCryptoHeader::try_from(crypto_header)?;

    let BuiltinCryptoFooter {
      common_mac,
      receiver_specific_macs,
    } = BuiltinCryptoFooter::try_from(crypto_footer)?;

    // Get decode key material
    let KeyMaterial_AES_GCM_GMAC {
      transformation_kind: key_material_transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      receiver_specific_key_id,
      master_receiver_specific_key,
    } = self
      .get_decode_key_materials(&sending_datareader_crypto_handle)
      // Get the one for submessages (not only payload)
      .map(KeyMaterial_AES_GCM_GMAC_seq::key_material)?;

    // Check that the key material matches the header. This should be redundant if
    // the method is called after preprocess_secure_submessage
    if !transformation_key_id.eq(sender_key_id) {
      Err(security_error!(
        "The key IDs don't match. The key has sender_key_id {}, while the header has \
         transformation_key_id {}",
        sender_key_id,
        transformation_key_id
      ))?;
    } else if !header_transformation_kind.eq(key_material_transformation_kind) {
      Err(security_error!(
        "The transformation_kind don't match. The key material has {:?}, while the header has {:?}",
        key_material_transformation_kind,
        header_transformation_kind
      ))?;
    }

    // Get the receiver-specific MAC if one is expected
    let receiver_specific_mac =
      find_receiver_specific_mac(*receiver_specific_key_id, &receiver_specific_macs)?;

    // TODO use session key?
    let decode_key = master_sender_key;
    // TODO use session key?
    let receiver_specific_key = master_receiver_specific_key;

    match key_material_transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        decode_datareader_submessage_gmac(
          decode_key,
          receiver_specific_key,
          initialization_vector,
          encoded_submessage,
          common_mac,
          receiver_specific_mac,
        )
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
        decode_datareader_submessage_gmac(
          decode_key,
          receiver_specific_key,
          initialization_vector,
          encoded_submessage,
          common_mac,
          receiver_specific_mac,
        )
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        decode_datareader_submessage_gcm(
          decode_key,
          receiver_specific_key,
          initialization_vector,
          encoded_submessage,
          common_mac,
          receiver_specific_mac,
        )
      }
    }
  }

  fn decode_serialized_payload(
    &self,
    encoded_buffer: Vec<u8>,
    _inline_qos: ParameterList,
    _receiving_datareader_crypto_handle: DatareaderCryptoHandle,
    sending_datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<Vec<u8>> {
    //TODO: this is only a mock implementation

    // Deserialize crypto header
    let (read_result, bytes_consumed) = CryptoHeader::read_with_length_from_buffer(&encoded_buffer);
    let data = encoded_buffer.split_at(bytes_consumed).1;
    let BuiltinCryptoHeader {
      transform_identifier:
        BuiltinCryptoTransformIdentifier {
          transformation_kind,
          transformation_key_id,
        },
      builtin_crypto_header_extra: BuiltinCryptoHeaderExtra(initialization_vector),
    } = read_result
      .map_err(|e| security_error!("Error while deserializing CryptoHeader: {}", e))
      .and_then(BuiltinCryptoHeader::try_from)?;

    // Get the payload decode key material
    let decode_key_material = self
      .get_decode_key_materials(&sending_datawriter_crypto_handle)
      .map(KeyMaterial_AES_GCM_GMAC_seq::payload_key_material)?;

    // Check that the key IDs match
    if decode_key_material.sender_key_id != transformation_key_id {
      return Err(security_error!(
        "Mismatched decode key IDs: the decoded CryptoHeader has {}, but the key associated with \
         the sending datawriter {} has {}.",
        transformation_key_id,
        sending_datawriter_crypto_handle,
        decode_key_material.sender_key_id
      ));
    }

    // Check that the transformation kind stays consistent
    if decode_key_material.transformation_kind != transformation_kind {
      return Err(security_error!(
        "Mismatched transformation kinds: the decoded CryptoHeader has {:?}, but the key material \
         associated with the sending datawriter {} has {:?}.",
        transformation_kind,
        sending_datawriter_crypto_handle,
        decode_key_material.transformation_kind
      ));
    }

    // TODO use session key?
    let decode_key = &decode_key_material.master_sender_key;

    match transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        decode_serialized_payload_gmac(decode_key, initialization_vector, data)
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
        decode_serialized_payload_gmac(decode_key, initialization_vector, data)
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        decode_serialized_payload_gcm(decode_key, initialization_vector, data)
      }
    }
  }
}
