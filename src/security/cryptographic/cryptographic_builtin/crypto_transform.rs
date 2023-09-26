use bytes::Bytes;
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
    secure_body::SecureBody,
    secure_postfix::SecurePostfix,
    secure_prefix::SecurePrefix,
    secure_rtps_postfix::SecureRTPSPostfix,
    secure_rtps_prefix::SecureRTPSPrefix,
    submessage::SecuritySubmessage,
    submessage_flag::FromEndianness,
    submessages::InterpreterSubmessage,
  },
  rtps::{Message, Submessage, SubmessageBody},
  security::cryptographic::cryptographic_builtin::*,
  security_error,
};
use super::{
  decode::{decode_submessage_gcm, decode_submessage_gmac, find_receiver_specific_mac},
  encode::{encode_gcm, encode_gmac},
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
    let EncodeSessionMaterials {
      key_id,
      transformation_kind,
      session_key,
      initialization_vector,
      receiver_specific_keys,
    } = self.session_encoding_materials(
      sending_endpoint_crypto_handle,
      KeyMaterialScope::MessageOrSubmessage,
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
}

impl CryptoTransform for CryptographicBuiltin {
  fn encode_serialized_payload(
    &self,
    plain_buffer: Vec<u8>,
    sending_datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<(Vec<u8>, ParameterList)> {
    // Get the key material for encrypting serialized payloads
    let EncodeSessionMaterials {
      key_id,
      transformation_kind,
      session_key,
      initialization_vector,
      ..
    } = self.session_encoding_materials(
      sending_datawriter_crypto_handle,
      KeyMaterialScope::PayloadOnly,
      &[],
    )?;

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
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        return Ok((plain_buffer, ParameterList::new()))
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
        let mac = aes_gcm_gmac::compute_mac(&session_key, initialization_vector, &plain_buffer)?;
        (plain_buffer, BuiltinCryptoFooter::only_common_mac(mac))
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        let (ciphertext, mac) =
          aes_gcm_gmac::encrypt(&session_key, initialization_vector, &plain_buffer)?;
        (ciphertext, BuiltinCryptoFooter::only_common_mac(mac))
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
    let EncodeSessionMaterials {
      key_id,
      transformation_kind,
      session_key,
      initialization_vector,
      receiver_specific_keys,
    } = self.session_encoding_materials(
      sending_participant_crypto_handle,
      KeyMaterialScope::MessageOrSubmessage,
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
    Message {
      header: rtps_header,
      submessages,
    }: Message,
    _receiving_participant_crypto_handle: ParticipantCryptoHandle,
    sending_participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<DecodeOutcome<Message>> {
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
      let decode_key_material = match self.get_session_decode_crypto_materials(
        sending_participant_crypto_handle,
        transformation_key_id,
        KeyMaterialScope::MessageOrSubmessage,
        initialization_vector,
      ){
        Some(decode_key_material)=>decode_key_material,
        None=> return Ok(DecodeOutcome::KeysNotFound(transformation_key_id))
      };

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
      let receiver_specific_key_and_mac =decode_key_material.receiver_specific_key
      // If the key is None, we are not expecting a receiver-specific MAC
      .map(|receiver_specific_key|find_receiver_specific_mac(
        &receiver_specific_key,
        &receiver_specific_macs,
      )).transpose()?;

      let decode_key = decode_key_material.session_key;

      match decode_key_material.transformation_kind {
        BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE
        | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
        | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
          // Validate signature even if it is not requested to avoid
          // unauthorized data injection attack.
          if decode_key_material.transformation_kind ==
            BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE {
            warn!("decode_rtps_message with crypto transformation kind = none. \
              Does not make sense, but validating MAC anyway.");
          }
          let submessages_with_info_source = encoded_content; // rename for clarity
          // We expect an InfoSource submessage followed by the original message
          if let
            [ Submessage { body: SubmessageBody::Interpreter(
                InterpreterSubmessage::InfoSource(info_source, _)), .. },
              submessages @ ..  // this is all the rest of the submessages
            ] = submessages_with_info_source
          {
            // Get original serialized data for submessage sequence: Concatenate original_bytes.
            let serialized_submessages = submessages_with_info_source.iter()
              .fold(Vec::<u8>::with_capacity(512), move |mut a,s| {
                a.extend_from_slice(s.original_bytes.as_ref().unwrap_or(&Bytes::new()).as_ref()); a }
              );
            // Validate the common MAC
            aes_gcm_gmac::validate_mac(&decode_key, initialization_vector, &serialized_submessages, common_mac)
              // Validate the receiver-specific MAC if one exists
              .and_then( |()|
                // common mac was ok, let's see if there is receiver-specific MAC
                receiver_specific_key_and_mac
                  .map(|(key,mac)|
                    aes_gcm_gmac::validate_mac( &key,initialization_vector, &serialized_submessages, mac ) )
                  // this is :Option<SecurityResult<()>>
                  .transpose(),
                  // to SecurityResult<Option<()>>, so we catch Err, if any.
                  // Value Some(()) means receiver-specific mac was ok, None means it was not required
                )
              // If the MACs are ok, return content. 
              .map( |_| (Vec::from(submessages), *info_source))
          } else {
            Err(security_error!("Expected the first submessage to be InfoSource."))
          }
        }
        BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
        | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
          // We expect a SecureBody submessage containing the encrypted message
          if let [ Submessage { body: SubmessageBody::Security(SecuritySubmessage::SecureBody(
                    SecureBody { crypto_content: CryptoContent { data: ciphertext },}, _ )), ..  }
            ] = encoded_content
          {
            // Validate the receiver-specific MAC if one exists, and exit on error
            if let Some((key,mac)) = receiver_specific_key_and_mac {
              aes_gcm_gmac::validate_mac( &key, initialization_vector, ciphertext, mac)?;
            }
            // Authenticated decryption, or exit on failure
            let mut plaintext =
              Bytes::copy_from_slice(
                &aes_gcm_gmac::decrypt(&decode_key, initialization_vector, ciphertext, common_mac)?);

            // We expect an InfoSource submessage followed by the original submessage sequence
            let info_source =
              if let Some(Submessage {body: SubmessageBody::Interpreter(
                    InterpreterSubmessage::InfoSource(info_source, _)), .. })
                  = Submessage::read_from_buffer(&mut plaintext)
                      .map_err(|e| security_error!("Failed to deserialize the plaintext: {e}"))?
              {
                info_source
              } else {
                Err(security_error!("Expected the first decrypted submessage to be InfoSource."))?
              };

            let mut submessages = Vec::<Submessage>::new();
            while !plaintext.is_empty() {
              if let Some(submessage) = Submessage::read_from_buffer(&mut plaintext)
                .map_err(|e| security_error!("Failed to deserialize the plaintext: {e}"))?
              {
                submessages.push(submessage);
              }
            }

            Ok((submessages, info_source))
          } else {
            Err(security_error!("Expected only a SecureBody submessage."))
          }
        }
      }
      .and_then( |(submessages, info_source)| {
        if InfoSource::from(rtps_header) == info_source {
          Ok(DecodeOutcome::Success(Message { header: rtps_header, submessages }))
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

  fn decode_submessage(
    &self,
    encoded_rtps_submessage: (SecurePrefix, Submessage, SecurePostfix),
    _receiving_local_participant_crypto_handle: ParticipantCryptoHandle,
    sending_remote_participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<DecodeOutcome<DecodedSubmessage>> {
    // Destructure header and footer
    let (SecurePrefix { crypto_header }, encoded_submessage, SecurePostfix { crypto_footer }) =
      encoded_rtps_submessage;

    let BuiltinCryptoHeader {
      transform_identifier:
        BuiltinCryptoTransformIdentifier {
          transformation_kind: header_transformation_kind,
          transformation_key_id: header_key_id,
        },
      builtin_crypto_header_extra: BuiltinCryptoHeaderExtra(initialization_vector),
    } = BuiltinCryptoHeader::try_from(crypto_header)?;

    let BuiltinCryptoFooter {
      common_mac,
      receiver_specific_macs,
    } = BuiltinCryptoFooter::try_from(crypto_footer)?;

    // Search for matching key materials over endpoints registered to the sender
    let sending_participant_endpoints = self
      .participant_to_endpoint_info
      .get(&sending_remote_participant_crypto_handle)
      .ok_or_else(|| {
        security_error!(
          "Could not find registered entities for the sending_remote_participant_crypto_handle {}",
          sending_remote_participant_crypto_handle
        )
      })?;

    let (matching_decode_materials, sending_endpoint_infos): (Vec<_>, Vec<EndpointInfo>) =
      sending_participant_endpoints
        .iter()
        .filter_map(|sending_endpoint_info| {
          self
            .get_session_decode_crypto_materials(
              sending_endpoint_info.crypto_handle,
              header_key_id,
              KeyMaterialScope::MessageOrSubmessage,
              initialization_vector,
            )
            .map(|decode_materials| (decode_materials, sending_endpoint_info))
        })
        .unzip();

    let decode_key = matching_decode_materials
      .iter()
      // Check that for all matched ids transformation kind matches the header
      .map(
        |DecodeSessionMaterials {
           transformation_kind,
           session_key,
           ..
         }| {
          if transformation_kind.eq(&header_transformation_kind) {
            Ok(session_key)
          } else {
            Err(security_error!(
              "Transformation kind of the submessage header does not match the key: expected \
               {:?}, received {:?}.",
              transformation_kind,
              header_transformation_kind
            ))
          }
        },
      )
      // Make sure the key is unambiguous
      .reduce(|accumulator, session_key_result| {
        accumulator.and_then(|acc_session_key| {
          session_key_result.and_then(|current_session_key| {
            if acc_session_key.eq(current_session_key) {
              Ok(acc_session_key)
            } else {
              Err(security_error!(
                "Multiple different matching decode keys found for the key id {:?} for the remote \
                 participant {}",
                header_key_id,
                sending_remote_participant_crypto_handle
              ))
            }
          })
        })
      });

    let decode_key = match decode_key {
      Some(key_result) => key_result?,
      None => {
        return Ok(DecodeOutcome::KeysNotFound(header_key_id));
      }
    };

    let receiver_specific_keys_and_macs = SecurityResult::<Vec<_>>::from_iter(
      matching_decode_materials
        .iter()
        .filter_map(
          |DecodeSessionMaterials {
             receiver_specific_key,
             ..
           }| receiver_specific_key.as_ref(),
        )
        .map(|receiver_specific_key| {
          find_receiver_specific_mac(receiver_specific_key, &receiver_specific_macs)
        }),
    )?;

    let decoded_submessage = match header_transformation_kind {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => {
        // Does this even make sense?
        warn!("Decode submessage success, but crypto transformation kind is none.");

        encoded_submessage.body
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => {
        decode_submessage_gmac(
          decode_key,
          initialization_vector,
          &encoded_submessage,
          common_mac,
          &receiver_specific_keys_and_macs,
        )?; // return verify error here, or continue
        encoded_submessage.body // it was plaintext anyway.
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        decode_submessage_gcm(
          decode_key,
          initialization_vector,
          &encoded_submessage,
          common_mac,
          &receiver_specific_keys_and_macs,
        )?
      }
    };

    match decoded_submessage {
      SubmessageBody::Writer(writer_submessage) => {
        let matching_readers = SecurityResult::from_iter(
          sending_endpoint_infos.iter().filter_map(
            |EndpointInfo {
               crypto_handle: remote_endpoint_crypto_handle,
               kind,
             }| match kind {
              EndpointKind::DataWriter => Some(
                self
                  .matched_local_endpoint
                  .get(remote_endpoint_crypto_handle)
                  .copied()
                  .ok_or_else(|| {
                    security_error!(
                      "The local reader matched to the remote writer crypto handle {} is missing.",
                      remote_endpoint_crypto_handle
                    )
                  }),
              ),
              _ => None,
            },
          ),
        )?;
        Ok(DecodeOutcome::Success(DecodedSubmessage::Writer(
          writer_submessage,
          matching_readers,
        )))
      }
      SubmessageBody::Reader(reader_submessage) => {
        let matching_writers = SecurityResult::from_iter(
          sending_endpoint_infos.iter().filter_map(
            |EndpointInfo {
               crypto_handle: remote_endpoint_crypto_handle,
               kind,
             }| match kind {
              EndpointKind::DataReader => Some(
                self
                  .matched_local_endpoint
                  .get(remote_endpoint_crypto_handle)
                  .copied()
                  .ok_or_else(|| {
                    security_error!(
                      "The local writer matched to the remote reader crypto handle {} is missing.",
                      remote_endpoint_crypto_handle
                    )
                  }),
              ),
              _ => None,
            },
          ),
        )?;
        Ok(DecodeOutcome::Success(DecodedSubmessage::Reader(
          reader_submessage,
          matching_writers,
        )))
      }

      SubmessageBody::Interpreter(_) => Err(security_error!(
        "Interpreter submessage after successful submessage decryption. This is not in the \
         specification."
      )),
      SubmessageBody::Security(_) => Err(security_error!(
        "Security submessage after successful submessage decryption."
      )),
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
    // .read_from_buffer() does not need endianness, because BuiltinCryptoHeader
    // only contains byte-oriented data, which is insensitive to endianness.

    let BuiltinCryptoFooter { common_mac, .. } =
      BuiltinCryptoFooter::read_from_buffer(footer_bytes)?;

    // Get the payload decode key material
    let decode_key_material = self.session_decode_crypto_materials(
      sending_datawriter_crypto_handle,
      transformation_key_id,
      KeyMaterialScope::PayloadOnly,
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
          warn!(
            "decode_serialized_payload with crypto transformation kind = none. Does not make \
             sense, but validating MAC anyway."
          );
        }
        aes_gcm_gmac::validate_mac(decode_key, initialization_vector, content_bytes, common_mac)
          // if validate_mac succeeds, then map result to content bytes
          .map(|()| Vec::from(content_bytes))
      }
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM
      | BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => {
        aes_gcm_gmac::decrypt(decode_key, initialization_vector, content_bytes, common_mac)
      }
    }
  }
}
