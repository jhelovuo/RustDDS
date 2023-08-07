use crate::{
  messages::submessages::{elements::crypto_content::CryptoContent, secure_body::SecureBody},
  rtps::Submessage,
  security::SecurityResult,
};
use super::{
  aes_gcm_gmac::{compute_mac, encrypt},
  types::{
    BuiltinCryptoFooter, BuiltinInitializationVector, BuiltinKey, BuiltinMAC, KeyLength,
    ReceiverKeyMaterial, ReceiverSpecificMAC,
  },
};

pub(super) fn encode_serialized_payload_gmac(
  key: &BuiltinKey,
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
  key: &BuiltinKey,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
) -> SecurityResult<(Vec<u8>, BuiltinCryptoFooter)> {
  // Compute ciphertext and MAC and return them with MAC wrapped in footer
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

fn compute_receiver_specific_macs(
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
  receiver_specific_key_materials: &[ReceiverKeyMaterial],
  common_mac: BuiltinMAC,
) -> SecurityResult<BuiltinCryptoFooter> {
  // Iterate over receiver_specific_keys to compute receiver_specific_macs
  SecurityResult::from_iter(receiver_specific_key_materials.iter().map(
    // Destructure
    |ReceiverKeyMaterial {
       receiver_specific_key_id,
       master_receiver_specific_key,
     }| {
      // Compute MAC
      compute_mac(
        master_receiver_specific_key,
        key_length,
        initialization_vector,
        data,
      )
      // Combine with id
      .map(|receiver_mac| ReceiverSpecificMAC {
        receiver_mac_key_id: *receiver_specific_key_id,
        receiver_mac,
      })
    },
  ))
  // Wrap in footer
  .map(|receiver_specific_macs| BuiltinCryptoFooter {
    common_mac,
    receiver_specific_macs,
  })
}

pub(super) fn encode_gmac(
  key: &BuiltinKey,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
  receiver_specific_key_materials: &[ReceiverKeyMaterial],
) -> SecurityResult<BuiltinCryptoFooter> {
  // Compute the common_mac
  compute_mac(key, key_length, initialization_vector, data)
    // Compute compute_receiver_specific_macs and return footer
    .and_then(|common_mac| {
      compute_receiver_specific_macs(
        key_length,
        initialization_vector,
        data,
        receiver_specific_key_materials,
        common_mac,
      )
    })
}

pub(super) fn encode_gcm(
  key: &BuiltinKey,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
  receiver_specific_key_materials: &[ReceiverKeyMaterial],
) -> SecurityResult<(Submessage, BuiltinCryptoFooter)> {
  // Compute the common_mac
  encrypt(key, key_length, initialization_vector, data).and_then(|(ciphertext, common_mac)| {
    // Compute compute_receiver_specific_macs
    compute_receiver_specific_macs(
      key_length,
      initialization_vector,
      &ciphertext,
      receiver_specific_key_materials,
      common_mac,
    )
    .and_then(|footer| {
      // Wrap the ciphertext into a SecureBody submessage
      SecureBody {
        crypto_content: CryptoContent::from(ciphertext),
      }
      .create_submessage(speedy::Endianness::BigEndian) // 9.5.2.4 use BigEndian
      // Return the pair
      .map(|submessage| (submessage, footer))
    })
  })
}
