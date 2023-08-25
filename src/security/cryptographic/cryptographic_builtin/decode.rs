use bytes::Bytes;
use speedy::{Writable};

use crate::{
  messages::submessages::{
    elements::{crypto_content::CryptoContent,},
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
    BuiltinInitializationVector, BuiltinMAC,
    ReceiverSpecificMAC, 
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
