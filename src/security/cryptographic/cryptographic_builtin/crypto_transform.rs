use enumflags2::BitFlags;
use speedy::{Readable, Writable};
use log::warn;

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
    decode_submessage_gcm, decode_submessage_gmac,
    find_receiver_specific_mac,
  },
  encode::{
    encode_gcm, encode_gmac,
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
    // Serialize plaintext
    // TODO: Do we respect RTPS endianness here? I.e. used and flagged encodings
    // match?
    let plaintext = plain_rtps_submessage
      .write_to_vec()
      .map_err(|err| security_error!("Error converting Submessage to byte vector: {}", err))?;

    // Get the key material for encoding
    let EncryptSessionMaterials {
      key_id,
      transformation_kind,
      session_key,
      initialization_vector,
      receiver_specific_keys,
    } = self.session_encoding_materials(
      sending_endpoint_crypto_handle,
      false,
      receiving_endpoint_crypto_handle_list,
    )?;

    // Compute encoded submessage and footer

    let (encoded_submessage, crypto_footer) = match transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        return Ok(EncodedSubmessage::Unencoded(plain_rtps_submessage))
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => (
        plain_rtps_submessage,
        encode_gmac(
          &session_key,
          initialization_vector,
          &plaintext,
          &receiver_specific_keys,
        )?,
      ),
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => encode_gcm(
        &session_key,
        initialization_vector,
        &plaintext,
        &receiver_specific_keys,
      )?,
    };

    // Build crypto header and security prefix
    let prefix = SecurePrefix {
      crypto_header: CryptoHeader::from(BuiltinCryptoHeader {
        transform_identifier: BuiltinCryptoTransformIdentifier {
          transformation_kind,
          transformation_key_id: key_id,
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

  fn decode_submessage(
    &self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    receiving_endpoint_crypto_handle: DatareaderCryptoHandle,
    sending_endpoint_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<SubmessageBody> {
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
    let decode_key_material = self.session_decode_crypto_materials(
      sending_endpoint_crypto_handle,
      transformation_key_id,
      false,
      initialization_vector,
    )?;

    // Check that the key id matches the header. This should be redundant if the
    // method is called after preprocess_secure_submessage
    if transformation_key_id != decode_key_material.key_id {
      Err(security_error!(
        "The key IDs do not match. The key material has sender_key_id {}, while the header has \
         transformation_key_id {}",
        decode_key_material.key_id,
        transformation_key_id
      ))?;
    } else if header_transformation_kind != decode_key_material.transformation_kind {
      Err(security_error!(
        "The transformation_kind do not match. The key material has {:?}, while the header has \
         {:?}",
        decode_key_material.transformation_kind,
        header_transformation_kind
      ))?;
    }

    // Get the receiver-specific MAC if one is expected
    let receiver_specific_key_and_mac = find_receiver_specific_mac(
      decode_key_material.receiver_specific_key,
      &receiver_specific_macs,
    )?;

    let decode_key = decode_key_material.session_key;

    match decode_key_material.transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        // Does this even make sense?
        warn!("Decode submessage success, but crypto transformation kind is none.");
        Ok(encoded_submessage.body)
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
        decode_submessage_gmac(
          &decode_key,
          initialization_vector,
          &encoded_submessage,
          common_mac,
          receiver_specific_key_and_mac,
        )?; // return verify error here, or continue

        Ok(encoded_submessage.body) // it was plaintext anyway.
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        decode_submessage_gcm(
          &decode_key,
          initialization_vector,
          &encoded_submessage,
          common_mac,
          receiver_specific_key_and_mac,
        )
      }
    }
  }
}

impl CryptoTransform for CryptographicBuiltin {
  fn encode_serialized_payload(
    &self,
    plain_buffer: Vec<u8>,
    sending_datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<(Vec<u8>, ParameterList)> {
    // Get the key material for encrypting serialized payloads
    let EncryptSessionMaterials {
      key_id,
      transformation_kind,
      session_key,
      initialization_vector,
      ..
    } = self.session_encoding_materials(sending_datawriter_crypto_handle, true, &[])?;

    // Receiver specific (signing) keys are not used.
    //
    // DDS Security spec, Section9.5.3.3.1 Overview, Table 72:
    // [The encode_serialized_payload] operation shall always set the
    // receiver_specific_macs attribute in the CryptoFooter to the empty
    // sequence.

    let header = BuiltinCryptoHeader {
      transform_identifier: BuiltinCryptoTransformIdentifier {
        transformation_kind,
        transformation_key_id: key_id,
      },
      builtin_crypto_header_extra: initialization_vector.into(),
    };

    let (encoded_data, footer) = match transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
        if transformation_kind == 
          BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE {
          warn!("decode_serialized_payload with crypto transformation kind = none. \
            Does not make sense, but validating MAC anyway.");
        }
        let mac = aes_gcm_gmac::compute_mac(&session_key, initialization_vector, &plain_buffer)?;
        ( plain_buffer, BuiltinCryptoFooter::only_common_mac(mac) )
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        let (ciphertext, mac) = aes_gcm_gmac::encrypt(&session_key, initialization_vector, &plain_buffer)?;
        ( ciphertext, BuiltinCryptoFooter::only_common_mac(mac) )
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
      // TODO: If the payload was not data but key, then construct a key_hash
      // and return that to be appended to the InlineQoS of the outgoing DATA Submessage.
      // Encrypted payloads must have their key sent as hash only (in InlineQoS), never plaintext.
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
    let EncryptSessionMaterials {
      key_id,
      transformation_kind,
      session_key,
      initialization_vector,
      receiver_specific_keys,
    } = self.session_encoding_materials(
      sending_participant_crypto_handle,
      false,
      &receiving_participant_crypto_handle_list,
    )?;

    // Compute encoded submessages and footer
    let (encoded_submessages, crypto_footer) = match transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        // TODO this is mainly for testing and debugging
        (
          submessages_with_info_source,
          encode_gmac(
            &session_key,
            initialization_vector,
            &plaintext,
            &receiver_specific_keys,
          )?,
        )
        /*  // TODO? switch to the following to avoid unnecessary pre/postfixes
        return Ok(EncodeResult::One(plain_rtps_message)); */
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => (
        submessages_with_info_source,
        encode_gmac(
          &session_key,
          initialization_vector,
          &plaintext,
          &receiver_specific_keys,
        )?,
      ),
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        encode_gcm(
          &session_key,
          initialization_vector,
          &plaintext,
          &receiver_specific_keys,
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
          transformation_key_id: key_id,
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
    Message { header: rtps_header, submessages }: Message,
    _receiving_participant_crypto_handle: ParticipantCryptoHandle,
    sending_participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<Message> {
    // we expect SecureRTPSPRefix + some submessages + SecureRTPSPostfix
    if let 
      [ Submessage { body:
          SubmessageBody::Security(SecuritySubmessage::SecureRTPSPrefix(
            SecureRTPSPrefix { crypto_header, .. }, _, )), .. },

        encoded_content @ .., 
        // ^ Note: This `..` is a "rest" pattern! Matches all submessages between first and last,

        Submessage { body:
          SubmessageBody::Security(SecuritySubmessage::SecureRTPSPostfix(
            SecureRTPSPostfix { crypto_footer }, _, )), .. }
      ] = submessages.as_slice()
    {
      let BuiltinCryptoHeader {
        transform_identifier:
          BuiltinCryptoTransformIdentifier {
            transformation_kind: header_transformation_kind,
            transformation_key_id,
          },
        builtin_crypto_header_extra: BuiltinCryptoHeaderExtra(initialization_vector),
      } = BuiltinCryptoHeader::try_from(crypto_header.clone())?;

      let BuiltinCryptoFooter { common_mac, receiver_specific_macs } 
        = BuiltinCryptoFooter::try_from(crypto_footer.clone())?;

      // Get decode key material
      let decode_key_material = self.session_decode_crypto_materials(
        sending_participant_crypto_handle,
        transformation_key_id,
        false,
        initialization_vector,
      )?;

      // Check that the key id matches the header
      if transformation_key_id != decode_key_material.key_id {
        Err(security_error!(
          "The key IDs don't match. The key material has sender_key_id {}, while the header has \
           transformation_key_id {}",
          decode_key_material.key_id,
          transformation_key_id
        ))?;
      } else if header_transformation_kind != decode_key_material.transformation_kind {
        Err(security_error!(
          "The transformation_kind don't match. The key material has {:?}, while the header has \
           {:?}",
          decode_key_material.transformation_kind,
          header_transformation_kind
        ))?;
      }

      // Get the receiver-specific MAC if one is expected
      let receiver_specific_key_and_mac = find_receiver_specific_mac(
        decode_key_material.receiver_specific_key,
        &receiver_specific_macs,
      )?;

      let decode_key = decode_key_material.session_key;

      match decode_key_material.transformation_kind {
        BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE
        | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
        | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
          // Validate signature even if it is not requested to avoid
          // unauthorized data injection attack.
          if decode_key_material.transformation_kind == 
            BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE {
            warn!("decode_serialized_payload with crypto transformation kind = none. \
              Does not make sense, but validating MAC anyway.");
          }
          decode_rtps_message_gmac(
            &decode_key,
            initialization_vector,
            encoded_content,
            common_mac,
            receiver_specific_key_and_mac,
          )
        }
        BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
        | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
          decode_rtps_message_gcm(
            &decode_key,
            initialization_vector,
            encoded_content,
            common_mac,
            receiver_specific_key_and_mac,
          )
        }
      }
      .and_then( |(submessages, info_source)| {
        if InfoSource::from(rtps_header) == info_source {
          Ok(Message { header: rtps_header, submessages })
        } else {
          Err(security_error!(
            "The RTPS header did not match the encoded InfoSource: {:?} expected to match {:?}",
            info_source, rtps_header))
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
    receiving_datareader_crypto_handle: DatawriterCryptoHandle,
    sending_datawriter_crypto_handle: DatareaderCryptoHandle,
  ) -> SecurityResult<WriterSubmessage> {
    match self.decode_submessage(
      encoded_rtps_submessage,
      receiving_datareader_crypto_handle,
      sending_datawriter_crypto_handle,
    )? {
      SubmessageBody::Writer(subm) => Ok(subm),
      other => {
        warn!("Expected WriterSubmessage, but decoded as {other:?}");
        Err(security_error(
          "decode_datawriter_submessage: Decode result was not WriterSubmessage",
        ))
      }
    }
  }

  fn decode_datareader_submessage(
    &self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    receiving_datawriter_crypto_handle: DatawriterCryptoHandle,
    sending_datareader_crypto_handle: DatareaderCryptoHandle,
  ) -> SecurityResult<ReaderSubmessage> {
    match self.decode_submessage(
      encoded_rtps_submessage,
      receiving_datawriter_crypto_handle,
      sending_datareader_crypto_handle,
    )? {
      SubmessageBody::Reader(subm) => Ok(subm),
      other => {
        warn!("Expected ReaderSubmessage, but decoded as {other:?}");
        Err(security_error(
          "decode_datareader_submessage: Decode result was not ReaderSubmessage",
        ))
      }
    }
  }

  fn decode_serialized_payload(
    &self,
    crypto_header_content_footer_buffer: Vec<u8>,
    _inline_qos: ParameterList,
    _receiving_datareader_crypto_handle: DatareaderCryptoHandle,
    sending_datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<Vec<u8>> {
    // According to DDS Security spec v1.1 Section
    // "9.5.3.3.4.4 Result from encode_serialized_payload"
    // the incoming data buffer is either
    //
    // CryptoHeader + SerializedPayload + CryptoFooter  (only signed)
    // or
    // CryptoHeader + CryptoContent + CryptoFooter
    //
    // We can detect which one it is from CryptoHeader contents.
    // splitting to the three parts has to be done by byte offset, because
    // SerializedPayload does not have a length marker, but both header and footer
    // have a fixed length. Footer is not allowed to have receiver specific MACs
    // here, which makes its size fixed.

    let head_len = BuiltinCryptoHeader::serialized_len();
    let foot_len = BuiltinCryptoFooter::minimal_serialized_len();

    // check length so that following split do not panic and subtract does not
    // underflow
    if crypto_header_content_footer_buffer.len() < head_len + foot_len + 4 {
      return Err(security_error("Encoded payload smaller than minimum size"));
    }
    let (header_bytes, content_and_footer_bytes) =
      crypto_header_content_footer_buffer.split_at(head_len);
    let (content_bytes, footer_bytes) =
      content_and_footer_bytes.split_at(content_and_footer_bytes.len() - foot_len);

    // Deserialize crypto header and footer
    let BuiltinCryptoHeader {
      transform_identifier:
        BuiltinCryptoTransformIdentifier {
          transformation_kind,
          transformation_key_id,
        },
      builtin_crypto_header_extra: BuiltinCryptoHeaderExtra(initialization_vector),
    } = BuiltinCryptoHeader::read_from_buffer(header_bytes)?;
    //TODO: Should this be read_from_buffer_with_ctx() to account for endianness?

    let BuiltinCryptoFooter { common_mac, .. } =
      BuiltinCryptoFooter::read_from_buffer(footer_bytes)?;

    // Get the payload decode key material
    let decode_key_material = self.session_decode_crypto_materials(
      sending_datawriter_crypto_handle,
      transformation_key_id,
      true,
      initialization_vector,
    )?;

    // Check that the key IDs match
    if decode_key_material.key_id != transformation_key_id {
      return Err(security_error!(
        "Mismatched decode key IDs: the decoded CryptoHeader has {}, but the key associated with \
         the sending datawriter {} has {}.",
        transformation_key_id,
        sending_datawriter_crypto_handle,
        decode_key_material.key_id
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

    let decode_key = &decode_key_material.session_key;

    match transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
        // Validate signature even if it is not requested to avoid
        // unauthorized data injection attack.
        if transformation_kind == BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE {
          warn!("decode_serialized_payload with crypto transformation kind = none. \
            Does not make sense, but validating MAC anyway.");
        }
        aes_gcm_gmac::validate_mac(decode_key, initialization_vector, content_bytes, common_mac)
          // if validate_mac succeeds, then map result to content bytes
          .map(|()| Vec::from(content_bytes) )
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        aes_gcm_gmac::decrypt(decode_key, initialization_vector, content_bytes, common_mac)
      }
    }
  }
}
