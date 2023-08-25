use bytes::Bytes;
use speedy::{Readable, Writable};

use crate::{
  messages::submessages::{
    elements::{crypto_content::CryptoContent, crypto_footer::CryptoFooter},
    info_source::InfoSource,
    secure_body::SecureBody,
    submessage::{InterpreterSubmessage, SecuritySubmessage, },
  },
  rtps::{Submessage, SubmessageBody},
  security::{SecurityError, SecurityResult},
  security_error,
};
use super::{
  aes_gcm_gmac::{decrypt, validate_mac},
  builtin_key::*,
  key_material::ReceiverSpecificKeyMaterial,
  types::{
    BuiltinCryptoContent, BuiltinCryptoFooter, BuiltinInitializationVector, BuiltinMAC,
    ReceiverSpecificMAC, MAC_LENGTH,
  },
};

pub(super) fn find_receiver_specific_mac(
  receiver_specific_key: Option<ReceiverSpecificKeyMaterial>,
  receiver_specific_macs: &[ReceiverSpecificMAC],
) -> SecurityResult<Option<(BuiltinKey, BuiltinMAC)>> {
  // If the key is None, we are not expecting a receiver-specific MAC
  receiver_specific_key.map( |ReceiverSpecificKeyMaterial{ key_id, key }|
    // Find the receiver-specific map by ID
    receiver_specific_macs
      .iter()
      .find( |ReceiverSpecificMAC { receiver_mac_key_id, .. }| *receiver_mac_key_id == key_id )
      .map(|ReceiverSpecificMAC { receiver_mac, .. }| ( key.clone() , *receiver_mac ) )
      // We are expecting to find a MAC, so reject if we do not
      .ok_or_else(|| security_error!( "No MAC found for receiver_specific_key_id {key_id}"))
  )
  .transpose()
}

pub(super) fn decode_serialized_payload_gmac(
  key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
) -> SecurityResult<Vec<u8>> {
  // The next submessage element should be the plaintext SerializedPayload,
  // followed by a CryptoFooter. However, serialized SerializedPayload
  // does not know its own length, but we know by 9.5.3.3.1 that the CryptoFooter
  // cannot have receiver-specific MACs in the case of encode_serialized_payload,
  // so the length of the footer is constant.
  // By 9.5.3.3.4.3 the footer consists of the common_mac, which is 16 bytes long,
  // and the length (0) of the reader-specific MAC sequence, which itself is a
  // 4-byte number, so the CDR-serialized length is
  let footer_length = MAC_LENGTH + 4;
  let serialized_payload_length = data
    .len()
    .checked_sub(footer_length)
    .ok_or(security_error!(
      "Bad data: the encoded buffer was too short to include a CryptoFooter."
    ))?;
  let (serialized_payload_data, crypto_footer_data) = data.split_at(serialized_payload_length);
  // Deserialize the footer to check its validity
  let BuiltinCryptoFooter { common_mac, .. } = CryptoFooter::read_from_buffer(crypto_footer_data)
    .map_err(|e| security_error!("Failed to deserialize the CryptoFooter: {}", e))
    .and_then(BuiltinCryptoFooter::try_from)?;

  // Validate MAC and deserialize serialized payload
  validate_mac(
    key,
    initialization_vector,
    serialized_payload_data,
    common_mac,
  )
  .and(Ok(Vec::from(serialized_payload_data)))
}

pub(super) fn decode_serialized_payload_gcm(
  key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
) -> SecurityResult<Vec<u8>> {
  // The next submessage element should be the CryptoContent containing the
  // encrypted SerializedPayload, followed by a CryptoFooter.

  // Deserialize crypto header
  let (read_result, bytes_consumed) = BuiltinCryptoContent::read_with_length_from_buffer(data);
  let footer_data = data.split_at(bytes_consumed).1;

  // Deserialize CryptoContent
  let BuiltinCryptoContent { data } =
    read_result.map_err(|e| security_error!("Error while deserializing CryptoContent: {}", e))?;

  // Deserialize CryptoFooter
  let BuiltinCryptoFooter { common_mac, .. } = CryptoFooter::read_from_buffer(footer_data)
    .map_err(|e| security_error!("Error while deserializing CryptoFooter: {}", e))
    .and_then(BuiltinCryptoFooter::try_from)?;

  // Decrypt serialized payload
  decrypt(key, initialization_vector, &data, common_mac)
}

pub(super) fn decode_submessage_gmac(
  key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  encoded_submessage: &Submessage,
  common_mac: BuiltinMAC,
  receiver_specific_key_and_mac: Option<(BuiltinKey, BuiltinMAC)>,
) -> SecurityResult<()> {
  // Serialize
  let data = encoded_submessage
    .write_to_vec()
    .map_err(|err| security_error!("Error converting Submessage to byte vector: {}", err))?;
  // TODO: This is insane.


  // Validate the common MAC
  validate_mac(key, initialization_vector, &data, common_mac)?;

  // Validate the receiver-specific MAC if one exists
  if let Some((receiver_specific_key, receiver_specific_mac)) = receiver_specific_key_and_mac {
    validate_mac(
      &receiver_specific_key,
      initialization_vector,
      &data,
      receiver_specific_mac,
    )?;
  }

  // both validations passed
  Ok(())
}

pub(super) fn decode_submessage_gcm(
  key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  encoded_submessage: &Submessage,
  common_mac: BuiltinMAC,
  receiver_specific_key_and_mac: Option<(BuiltinKey, BuiltinMAC)>,
) -> SecurityResult<SubmessageBody> {
  // Destructure to get the data
  match &encoded_submessage.body {
    SubmessageBody::Security(SecuritySubmessage::SecureBody(
      SecureBody {
        crypto_content: CryptoContent { data },
      },
      _,
    )) => {
      // Validate the receiver-specific MAC if one exists
      if let Some((receiver_specific_key, receiver_specific_mac)) = receiver_specific_key_and_mac {
        validate_mac(
          &receiver_specific_key,
          initialization_vector,
          data,
          receiver_specific_mac,
        )?;
      }

      // Authenticated decryption
      let mut plaintext =
        Bytes::copy_from_slice(&decrypt(key, initialization_vector, data, common_mac)?);

      // Deserialize (submessage deserialization is a bit funky atm)
      match Submessage::read_from_buffer(&mut plaintext)
        .map_err(|e| security_error!("Failed to deserialize the plaintext: {}", e))?
      {
        Some(Submessage { body, .. }) => Ok(body),
        None => Err(security_error!(
          "Failed to deserialize the plaintext into a submessage. It could have been PAD or \
           vendor-specific or otherwise unrecognized submessage kind."
        )),
      }
    }

    other => Err(security_error!(
      "When transformation kind is GCM, decode_datawriter_submessage expects a SecureBody, \
       received {other:?}")),
  }
}


pub(super) fn decode_rtps_message_gmac(
  key: &BuiltinKey,
  receiver_specific_key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  submessages_with_info_source: &[Submessage],
  common_mac: BuiltinMAC,
  receiver_specific_mac: Option<BuiltinMAC>,
) -> SecurityResult<(Vec<Submessage>, InfoSource)> {
  // We expect an InfoSource submessage followed by the original message
  if let [Submessage {
    body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoSource(info_source, _)),
    ..
  }, submessages @ ..] = submessages_with_info_source
  {
    // Serialize data for MAC validation
    SecurityResult::<Vec<Vec<u8>>>::from_iter(submessages_with_info_source.iter().map(
      |submessage| {
        submessage
          .write_to_vec()
          .map_err(|err| security_error!("Error converting Submessage to byte vector: {}", err))
      },
    ))
    .and_then(|serialized_submessages| {
      let data = serialized_submessages.concat();
      // Validate the common MAC
      validate_mac(key, initialization_vector, &data, common_mac)
        // Validate the receiver-specific MAC if one exists
        .and(
          receiver_specific_mac
            .map(|receiver_specific_mac| {
              validate_mac(
                receiver_specific_key,
                initialization_vector,
                &data,
                receiver_specific_mac,
              )
            })
            .transpose(),
        )
    })
    // If the MACs are ok, return
    .map(|_| (Vec::from(submessages), *info_source))
  } else {
    Err(security_error!(
      "Expected the first submessage to be InfoSource."
    ))
  }
}

pub(super) fn decode_rtps_message_gcm(
  key: &BuiltinKey,
  receiver_specific_key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  encrypted_submessages: &[Submessage],
  common_mac: BuiltinMAC,
  receiver_specific_mac: Option<BuiltinMAC>,
) -> SecurityResult<(Vec<Submessage>, InfoSource)> {
  // We expect a SecureBody submessage containing the encrypted message
  if let [Submessage {
    body:
      SubmessageBody::Security(SecuritySubmessage::SecureBody(
        SecureBody {
          crypto_content: CryptoContent { data },
        },
        _,
      )),
    ..
  }] = encrypted_submessages
  {
    // Validate the receiver-specific MAC if one exists
    if let Some(receiver_specific_mac) = receiver_specific_mac {
      validate_mac(
        receiver_specific_key,
        initialization_vector,
        data,
        receiver_specific_mac,
      )?;
    }

    // Authenticated decryption
    let mut plaintext =
      Bytes::copy_from_slice(&decrypt(key, initialization_vector, data, common_mac)?);

    // We expect an InfoSource submessage followed by the original message
    let info_source = if let Some(Submessage {
      body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoSource(info_source, _)),
      ..
    }) = Submessage::read_from_buffer(&mut plaintext)
      .map_err(|e| security_error!("Failed to deserialize the plaintext: {}", e))?
    {
      info_source
    } else {
      Err(security_error!(
        "Expected the first decrypted submessage to be InfoSource."
      ))?
    };

    let mut submessages = Vec::<Submessage>::new();
    while !plaintext.is_empty() {
      if let Some(submessage) = Submessage::read_from_buffer(&mut plaintext)
        .map_err(|e| security_error!("Failed to deserialize the plaintext: {}", e))?
      {
        submessages.push(submessage);
      }
    }

    Ok((submessages, info_source))
  } else {
    Err(security_error!("Expected only a SecureBody submessage."))
  }
}
