use ring::{aead::*, error::Unspecified};

use crate::security::SecurityResult;
use super::{
  builtin_key::*,
  types::{BuiltinInitializationVector, BuiltinMAC, MAC_LENGTH},
};

// By design of Secure RTPS, there is a unique Initialization Vector
// for each submessage, and we only encrypt once (one submessage) with that,
// so we can construct a trivial sequence of just one element.

struct TrivialNonceSequence {
  iv: BuiltinInitializationVector,
  used: bool, // The purpose of this is to panic on misuse.
}

impl TrivialNonceSequence {
  fn new(iv: BuiltinInitializationVector) -> Self {
    TrivialNonceSequence { iv, used: false }
  }
}

impl NonceSequence for TrivialNonceSequence {
  fn advance(&mut self) -> Result<Nonce, Unspecified> {
    if self.used {
      Err(Unspecified) // you had one nonce
    } else {
      self.used = true;
      Ok(Nonce::assume_unique_for_key(self.iv.into()))
    }
  }
}

// Generate a key of the given length
pub(super) fn keygen(key_length: KeyLength) -> BuiltinKey {
  BuiltinKey::generate_random(key_length)
}

// Generate a key of the given length
pub(super) fn try_keygen(key_length: Option<KeyLength>) -> BuiltinKey {
  match key_length {
    Some(key_length) => BuiltinKey::generate_random(key_length),
    None => BuiltinKey::ZERO,
  }
}

#[allow(non_snake_case)]
fn to_unbound_AES_GCM_key(key: &BuiltinKey) -> UnboundKey {
  match key {
    // unwraps should be safe, because builtin key lengths always match expected length
    BuiltinKey::AES128(key) => UnboundKey::new(&AES_128_GCM, key).unwrap(),
    BuiltinKey::AES256(key) => UnboundKey::new(&AES_256_GCM, key).unwrap(),
  }
}

fn to_builtin_mac(tag: &Tag) -> BuiltinMAC {
  // This .unwrap() cannot fail, as both have fixed length
  tag.as_ref().try_into().unwrap()
}

// Section "9.5.3.3.4.2 Format of the CryptoContent Submessage Element" :
// "Note that the cipher operations have 16-byte block-size and add padding when
// needed. Therefore the secure data.length (“N”) will always be a multiple of
// 16.""
//
// So we do not have to worry about adding padding here.
// We DO have to assume the ciphertext may be longer than the plaintext.

// Computes the message authentication code (MAC) for the given data
pub(super) fn compute_mac(
  key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
) -> SecurityResult<BuiltinMAC> {
  // ring encrypts + tags (signs) in place, so we must create a buffer for that.
  let mut in_out_data = Vec::from(data);

  // Section "9.5.3.3.4.4 Result from encode_serialized_payload" :
  //
  // "The common_mac in the CryptoFooter is the authentication tag generated
  // by the same AES-GCM where the Additional Authenticated Data is empty."

  let mut sealing_key = SealingKey::new(
    to_unbound_AES_GCM_key(key),
    TrivialNonceSequence::new(initialization_vector),
  );

  let tag = sealing_key.seal_in_place_separate_tag(Aad::empty(), &mut in_out_data)?;

  Ok(to_builtin_mac(&tag))
}

// Authenticated encryption: computes the ciphertext and and a MAC for it
pub(super) fn encrypt(
  key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  plaintext: &[u8],
) -> SecurityResult<(Vec<u8>, BuiltinMAC)> {
  // Compute the ciphertext
  // ring encrypts + tags (signs) in place, so we must create a buffer for that.
  let mut in_out_data = Vec::from(plaintext);

  let mut sealing_key = SealingKey::new(
    to_unbound_AES_GCM_key(key),
    TrivialNonceSequence::new(initialization_vector),
  );

  let tag = sealing_key.seal_in_place_separate_tag(Aad::empty(), &mut in_out_data)?;

  Ok((in_out_data, to_builtin_mac(&tag)))
}

// Computes the MAC for the data and compares it with the one provided
pub(super) fn validate_mac(
  key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
  mac: BuiltinMAC,
) -> SecurityResult<()> {
  let mut in_out = Vec::with_capacity(data.len() + MAC_LENGTH);
  in_out.extend_from_slice(data);
  in_out.extend_from_slice(&mac);

  let mut opening_key = OpeningKey::new(
    to_unbound_AES_GCM_key(key),
    TrivialNonceSequence::new(initialization_vector),
  );

  // This will return `Err(..)` if verification fails
  let plaintext = opening_key.open_in_place(Aad::empty(), &mut in_out)?;
  // If we get here, the mac ("tag") was valid.
  Ok(())
}

// Authenticated decryption: validates the MAC and decrypts the ciphertext
pub(super) fn decrypt(
  key: &BuiltinKey,
  initialization_vector: BuiltinInitializationVector,
  ciphertext: &[u8],
  mac: BuiltinMAC,
) -> SecurityResult<Vec<u8>> {
  // TODO: round data.len() up to block size
  let mut in_out = Vec::with_capacity(ciphertext.len() + MAC_LENGTH);
  in_out.extend_from_slice(ciphertext.as_ref());
  in_out.extend_from_slice(mac.as_ref());

  let mut opening_key = OpeningKey::new(
    to_unbound_AES_GCM_key(key),
    TrivialNonceSequence::new(initialization_vector),
  );

  // This will return `Err(..)` if verification fails
  let plaintext = opening_key.open_in_place(Aad::empty(), &mut in_out)?;
  // If we get here, the mac ("tag") was valid.
  // and `plaintext` is actually a slice of `in_out`
  let plain_len = plaintext.len();
  in_out.truncate(plain_len);

  Ok(in_out)
}
