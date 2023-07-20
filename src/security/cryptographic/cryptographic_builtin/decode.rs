use bytes::Bytes;
use speedy::Readable;

use crate::{
  messages::submessages::elements::{
    crypto_footer::CryptoFooter, serialized_payload::SerializedPayload,
  },
  security::{SecurityError, SecurityResult},
  security_error,
};
use super::{
  aes_gcm_gmac::{decrypt, validate_mac},
  types::{
    BuiltinCryptoContent, BuiltinCryptoFooter, BuiltinInitializationVector, KeyLength, MAC_LENGTH,
  },
};

pub(super) fn decode_serialized_payload_gmac(
  key: &Vec<u8>,
  key_length: &KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
) -> SecurityResult<SerializedPayload> {
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
    *key_length,
    initialization_vector,
    serialized_payload_data,
    common_mac,
  )
  .and(
    SerializedPayload::from_bytes(&Bytes::copy_from_slice(serialized_payload_data))
      .map_err(|e| security_error!("Failed to deserialize the SerializedPayload: {}", e)),
  )
}

pub(super) fn decode_serialized_payload_gcm(
  key: &Vec<u8>,
  key_length: &KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
) -> SecurityResult<SerializedPayload> {
  // The next submessage element should be the CryptoContent containing the
  // encrypted SerializedPayload, followed by a CryptoFooter.

  // Deserialize crypto header
  let (read_result, bytes_consumed) = BuiltinCryptoContent::read_with_length_from_buffer(data);
  let footer_data = data.split_at(bytes_consumed).1;

  let BuiltinCryptoContent { data } =
    read_result.map_err(|e| security_error!("Error while deserializing CryptoContent: {}", e))?;

  // Deserialize CryptoFooter
  let BuiltinCryptoFooter { common_mac, .. } = CryptoFooter::read_from_buffer(footer_data)
    .map_err(|e| security_error!("Error while deserializing CryptoFooter: {}", e))
    .and_then(BuiltinCryptoFooter::try_from)?;

  // Decrypt and deserialize serialized payload
  decrypt(key, *key_length, initialization_vector, data, common_mac).and_then(|plaintext| {
    SerializedPayload::from_bytes(&Bytes::copy_from_slice(&plaintext))
      .map_err(|e| security_error!("Failed to deserialize the SerializedPayload: {}", e))
  })
}
