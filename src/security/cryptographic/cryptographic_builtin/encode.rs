use crate::security::SecurityResult;
use super::{
  aes_gcm_gmac::{compute_mac, encrypt},
  types::{BuiltinCryptoFooter, BuiltinInitializationVector, KeyLength},
};

pub(super) fn encode_serialized_payload_gmac(
  key: &Vec<u8>,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
) -> SecurityResult<(Vec<u8>, BuiltinCryptoFooter)> {
  // Compute MAC and return it in the footer along the plaintext
  compute_mac(key, key_length, initialization_vector, data).map(|common_mac| {
    (
      Vec::from(data),
      BuiltinCryptoFooter {
        common_mac,
        receiver_specific_macs: Vec::new(),
      },
    )
  })
}

pub(super) fn encode_serialized_payload_gcm(
  key: &Vec<u8>,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
) -> SecurityResult<(Vec<u8>, BuiltinCryptoFooter)> {
  encrypt(key, key_length, initialization_vector, data).map(|(ciphertext, common_mac)| {
    (
      ciphertext,
      BuiltinCryptoFooter {
        common_mac,
        receiver_specific_macs: Vec::new(),
      },
    )
  })
}
