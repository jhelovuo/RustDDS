use bytes::Bytes;


use crate::{
  messages::submessages::{
    elements::crypto_content::CryptoContent,
    secure_body::SecureBody,
    submessage::{SecuritySubmessage},
  },
  rtps::{Submessage, SubmessageBody},
  security::{SecurityError, SecurityResult},
  security_error,
};
use super::{
  aes_gcm_gmac::{decrypt, validate_mac},
  builtin_key::*,
  key_material::ReceiverSpecificKeyMaterial,
  types::{BuiltinInitializationVector, BuiltinMAC, ReceiverSpecificMAC},
};

pub(super) fn find_receiver_specific_mac(
  receiver_specific_key: Option<ReceiverSpecificKeyMaterial>,
  receiver_specific_macs: &[ReceiverSpecificMAC],
) -> SecurityResult<Option<(BuiltinKey, BuiltinMAC)>> {
  // If the key is None, we are not expecting a receiver-specific MAC
  receiver_specific_key
    .map(|ReceiverSpecificKeyMaterial{ key_id, key }|
    // Find the receiver-specific map by ID
    receiver_specific_macs
      .iter()
      .find( |ReceiverSpecificMAC { receiver_mac_key_id, .. }| *receiver_mac_key_id == key_id )
      .map(|ReceiverSpecificMAC { receiver_mac, .. }| ( key.clone() , *receiver_mac ) )
      // We are expecting to find a MAC, so reject if we do not
      .ok_or_else(|| security_error!( "No MAC found for receiver_specific_key_id {key_id}")))
    .transpose()
}

pub(super) fn decode_submessage_gmac(
  key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  encoded_submessage: &Submessage,
  common_mac: BuiltinMAC,
  receiver_specific_key_and_mac: Option<(BuiltinKey, BuiltinMAC)>,
) -> SecurityResult<()> {
  
  let data = encoded_submessage.original_bytes.as_ref()
    .ok_or_else(|| security_error!("The dog ate my submessage bytes."))?;

  // Validate the common MAC
  validate_mac(key, initialization_vector, data, common_mac)?;

  // Validate the receiver-specific MAC if one exists
  if let Some((receiver_specific_key, receiver_specific_mac)) = receiver_specific_key_and_mac {
    validate_mac(
      &receiver_specific_key,
      initialization_vector,
      data,
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
      SecureBody { crypto_content: CryptoContent { data } }, _, )) => {
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
       received {other:?}"
    )),
  }
}
