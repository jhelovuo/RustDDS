use speedy::Writable;

use crate::{
  messages::{
    header::Header,
    protocol_id::ProtocolId,
    submessages::{
      elements::{
        crypto_content::CryptoContent, crypto_header::CryptoHeader, parameter_list::ParameterList,
        serialized_payload::SerializedPayload,
      },
      info_source::InfoSource,
      secure_postfix::SecurePostfix,
      secure_prefix::SecurePrefix,
      secure_rtps_prefix::SecureRTPSPrefix,
      submessage::{InterpreterSubmessage, SecuritySubmessage},
      submessages::{ReaderSubmessage, WriterSubmessage},
    },
  },
  rtps::{Message, Submessage, SubmessageBody},
  security::cryptographic::{builtin_types::*, cryptographic_builtin::*},
  security_error,
};

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

    let payload_key = self
      .encode_keys_
      .get(&sending_datawriter_crypto)
      .ok_or(security_error!(
        "Could not find keys for the handle {}",
        sending_datawriter_crypto
      ))
      .cloned()?
      .payload_key();

    let header = BuiltinCryptoHeader {
      transform_identifier: BuiltinCryptoTransformIdentifier {
        transformation_kind: payload_key.transformation_kind,
        transformation_key_id: 0,
      },
      session_id: [0, 0, 0, 0],
      initialization_vector_suffix: [0; 8],
    };
    match payload_key.transformation_kind {
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

  fn encode_datawriter_submessage(
    &mut self,
    plain_rtps_submessage: Submessage,
    sending_datawriter_crypto: DatawriterCryptoHandle,
    receiving_datareader_crypto_list: Vec<DatareaderCryptoHandle>,
  ) -> SecurityResult<EncodeResult<EncodedSubmessage>> {
    //TODO: this is only a mock implementation
    let key = self
      .encode_keys_
      .get(&sending_datawriter_crypto)
      .ok_or(security_error!(
        "Could not find keys for the handle {}",
        sending_datawriter_crypto
      ))
      .cloned()?
      .key();

    match key.transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => Ok(EncodeResult::One(
        EncodedSubmessage::Unencoded(plain_rtps_submessage),
      )),
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

    let key = self
      .encode_keys_
      .get(&sending_datareader_crypto)
      .ok_or(security_error!(
        "Could not find keys for the handle {}",
        sending_datareader_crypto
      ))
      .cloned()?
      .key();
    match key.transformation_kind {
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

    let key = self
      .encode_keys_
      .get(&sending_participant_crypto)
      .ok_or(security_error!(
        "Could not find keys for the handle {}",
        sending_participant_crypto
      ))
      .cloned()?
      .key();

    match key.transformation_kind {
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

    // Check that the first submessage is SecureRTPSPrefix
    if let Some((
      Submessage {
        body:
          SubmessageBody::Security(SecuritySubmessage::SecureRTPSPrefix(
            SecureRTPSPrefix {
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
      },
      submessages,
    )) = encoded_buffer.submessages.split_first()
    {
      // Check that the last submessage is a SecureRTPSPostfix
      if let Some((
        Submessage {
          body: SubmessageBody::Security(SecuritySubmessage::SecureRTPSPostfix(_, _)),
          ..
        },
        submessages,
      )) = submessages.split_last()
      {
        // Check the validity of transformation_kind
        let message_transformation_kind =
          BuiltinCryptoTransformationKind::try_from(*transformation_kind)?;

        match message_transformation_kind {
          BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
            // In this case we expect the encoded message to be of the following form:
            // SecureRTPSPrefix, (InfoSource containing the RTPS-header of
            // the original message), the original submessages, SecureRTPSPostfix,
            // where the InfoSource is optional
            if let Some((
              Submessage {
                body:
                  SubmessageBody::Interpreter(InterpreterSubmessage::InfoSource(
                    InfoSource {
                      protocol_version,
                      vendor_id,
                      guid_prefix,
                    },
                    _,
                  )),
                ..
              },
              submessages,
            )) = submessages.split_first()
            {
              Ok(Message {
                header: Header {
                  // Copy header information from InfoSource
                  protocol_id: ProtocolId::PROTOCOL_RTPS,
                  protocol_version: *protocol_version,
                  vendor_id: *vendor_id,
                  guid_prefix: *guid_prefix,
                },
                submessages: Vec::from(submessages),
              })
            } else {
              Ok(Message {
                // Use the same header information as the input
                header: encoded_buffer.header,
                submessages: Vec::from(submessages),
              })
            }
          }
          BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC => {
            // In this case we expect the encoded message to be of the following form:
            // SecureRTPSPrefix, InfoSource containing the RTPS-header of
            // the original message, the original submessages, SecureRTPSPostfix
            if let Some((
              Submessage {
                body:
                  SubmessageBody::Interpreter(InterpreterSubmessage::InfoSource(
                    InfoSource {
                      protocol_version,
                      vendor_id,
                      guid_prefix,
                    },
                    _,
                  )),
                ..
              },
              submessages,
            )) = submessages.split_first()
            {
              // TODO: Check the MACs

              Ok(Message {
                header: Header {
                  // Copy header information from InfoSource
                  protocol_id: ProtocolId::PROTOCOL_RTPS,
                  protocol_version: *protocol_version,
                  vendor_id: *vendor_id,
                  guid_prefix: *guid_prefix,
                },
                submessages: Vec::from(submessages),
              })
            } else {
              Err(security_error!(
                "Expected an InfoSource submessage after SecureRTPSPrefix."
              ))
            }
          }
          BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM => {
            // In this case we expect the encoded message to be of the following form:
            // SecureRTPSPrefix, SecureBody containing the encrypted message,
            // SecureRTPSPostfix
            todo!()
          }
          BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => todo!(),
          BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => todo!(),
        }
      } else {
        Err(security_error!(
          "When a message starts with a SecureRTPSPrefix, it is expected to end with a \
           SecureRTPSPostfix."
        ))
      }
    } else {
      // The first submessage is not a SecureRTPSPrefix, pass through
      Ok(encoded_buffer)
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
        if let Some(KeyMaterial_AES_GCM_GMAC {
          transformation_kind,
          sender_key_id,
          ..
        }) = self
          .decode_keys_
          .get(handle)
          .cloned()
          .map(KeyMaterial_AES_GCM_GMAC_seq::key)
        {
          // Compare keys to the crypto transform identifier
          if transformation_kind == submessage_transformation_kind
            && sender_key_id == transformation_key_id
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
      // No matching keys were found for any entity registered to the sender
      Err(security_error!(
        "Could not find matching keys for any registered entity for the \
         sending_participant_crypto {}.",
        sending_participant_crypto
      ))
    } else {
      Err(security_error!(
        "preprocess_secure_submsg expects encoded_rtps_submessage to be a SEC_PREFIX. Received \
         {:?}.",
        encoded_rtps_submessage.header.kind
      ))
    }
  }

  fn decode_datawriter_submessage(
    &mut self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    receiving_datareader_crypto: DatareaderCryptoHandle,
    sending_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<WriterSubmessage> {
    match self
      .decode_keys_
      .get(&sending_datawriter_crypto)
      .cloned()
      .map(KeyMaterial_AES_GCM_GMAC_seq::key)
    {
      Some(KeyMaterial_AES_GCM_GMAC {
        transformation_kind: BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE,
        ..
      }) => match encoded_rtps_submessage.1.body {
        SubmessageBody::Writer(writer_submessage) => Ok(writer_submessage),
        other => Err(security_error!(
          "When transformation kind is CRYPTO_TRANSFORMATION_KIND_NONE, \
           decode_datawriter_submessage expects a WriterSubmessage, received {:?}",
          other
        )),
      },

      None => Err(security_error!(
        "No decode keys found for the remote datawriter {}",
        sending_datawriter_crypto
      )),
      _ => todo!(),
    }
  }

  fn decode_datareader_submessage(
    &mut self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    receiving_datawriter_crypto: DatawriterCryptoHandle,
    sending_datareader_crypto: DatareaderCryptoHandle,
  ) -> SecurityResult<ReaderSubmessage> {
    match self
      .decode_keys_
      .get(&sending_datareader_crypto)
      .cloned()
      .map(KeyMaterial_AES_GCM_GMAC_seq::key)
    {
      Some(KeyMaterial_AES_GCM_GMAC {
        transformation_kind: BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE,
        ..
      }) => match encoded_rtps_submessage.1.body {
        SubmessageBody::Reader(reader_submessage) => Ok(reader_submessage),
        other => Err(security_error!(
          "When transformation kind is CRYPTO_TRANSFORMATION_KIND_NONE, \
           decode_datareader_submessage expects a ReaderSubmessage, received {:?}",
          other
        )),
      },
      None => Err(security_error!(
        "No decode keys found for the remote datawriter {}",
        sending_datareader_crypto
      )),
      _ => todo!(),
    }
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
