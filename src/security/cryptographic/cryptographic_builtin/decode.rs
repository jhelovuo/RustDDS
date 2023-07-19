use bytes::Bytes;
use speedy::Readable;

use crate::{
  messages::submessages::elements::{
    crypto_footer::CryptoFooter, serialized_payload::SerializedPayload,
  },
  security::{SecurityError, SecurityResult},
  security_error,
};
use super::types::{KeyLength, MAC_LENGTH};

pub(super) fn decode_serialized_payload_gmac(
  data: &[u8],
  key_length: &KeyLength,
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
  let crypto_footer = CryptoFooter::read_from_buffer(crypto_footer_data)
    .map_err(|e| security_error!("Failed to deserialize the CryptoFooter: {}", e))?;

  match key_length {
    KeyLength::None => {
      SerializedPayload::from_bytes(&Bytes::copy_from_slice(serialized_payload_data))
        .map_err(|e| security_error!("Failed to deserialize the SerializedPayload: {}", e))
    }
    KeyLength::AES128 => {
      todo!() // TODO check MACs
    }
    KeyLength::AES256 => {
      todo!() // TODO check MACs
    }
  }
}
