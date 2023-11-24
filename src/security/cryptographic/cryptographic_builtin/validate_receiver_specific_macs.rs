#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::security::cryptographic::CryptoTransformKeyId;
use super::{
  aes_gcm_gmac::validate_mac,
  key_material::ReceiverSpecificKeyMaterial,
  types::{BuiltinInitializationVector, BuiltinMAC, ReceiverSpecificMAC},
  DecodeSessionMaterials,
};

fn find_receiver_specific_mac<'a>(
  receiver_specific_key_id: &CryptoTransformKeyId,
  receiver_specific_macs: &'a [ReceiverSpecificMAC],
) -> Option<&'a BuiltinMAC> {
  receiver_specific_macs
    .iter()
    .find(
      |ReceiverSpecificMAC {
         receiver_mac_key_id,
         ..
       }| receiver_mac_key_id.eq(receiver_specific_key_id),
    )
    .map(|ReceiverSpecificMAC { receiver_mac, .. }| receiver_mac)
}

pub(super) fn validate_receiver_specific_mac(
  decode_materials: &DecodeSessionMaterials,
  initialization_vector: &BuiltinInitializationVector,
  common_mac: &BuiltinMAC,
  receiver_specific_macs: &[ReceiverSpecificMAC],
) -> bool {
  if let DecodeSessionMaterials {
    receiver_specific_key: Some(ReceiverSpecificKeyMaterial { key_id, key }),
    ..
  } = decode_materials
  {
    if let Some(receiver_specific_mac) = find_receiver_specific_mac(key_id, receiver_specific_macs)
    {
      // The receiver-specific MAC is computed for common_mac, not the  ciphertext.
      // See 9.5.3.3.4
      validate_mac(
        key,
        *initialization_vector,
        common_mac,
        *receiver_specific_mac,
      )
      .map_or_else(
        |e| {
          error!("Decoding receiver-specific MAC failed: {e}");
          false
        },
        |_| true, // Success
      )
    } else {
      trace!(
        "No receiver-specific MAC found for the receiver-specific key id {:?}, rejecting.",
        key_id
      );
      false
    }
  } else {
    // No receiver specific key so no receiver specific MAC expected
    true
  }
}
