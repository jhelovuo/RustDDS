use crate::{
  security::{SecurityError, SecurityResult},
  security_error,
};
use super::types::{
  AES128Key, AES256Key, BuiltinInitializationVector, BuiltinMAC, KeyLength, MAC_LENGTH,
};

#[doc(hidden)]
#[macro_export]
// A shorthand to a closure that replaces an error with one that compares the
// key lengths
macro_rules! key_length_error {
  ($key:expr,$key_length:expr) => {
    |_| {
      security_error!(
        "Got a {} bytes long key, expected {:?}.",
        $key.len(),
        $key_length
      )
    }
  };
}

#[doc(hidden)]
#[macro_export]
// Converts the key to AES128Key and saves it in a variable, sending an error if
// the conversion fails
macro_rules! convert_to_AES128_key {
  ($key:expr,$key_length:expr,$variable_name:ident) => {
    let $variable_name =
      AES128Key::try_from($key.clone()).map_err(key_length_error!($key, $key_length))?;
  };
}
#[doc(hidden)]
#[macro_export]
// Converts the key to AES256Key and saves it in a variable, sending an error if
// the conversion fails
macro_rules! convert_to_AES256_key {
  ($key:expr,$key_length:expr,$variable_name:ident) => {
    let $variable_name =
      AES256Key::try_from($key.clone()).map_err(key_length_error!($key, $key_length))?;
  };
}

// Computes the message authentication code (MAC) for the given data
pub(super) fn compute_mac(
  key: &Vec<u8>,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
) -> SecurityResult<BuiltinMAC> {
  match key_length {
    // If no authentication is done, the MAC shall be all zeros
    KeyLength::None => Ok([0; MAC_LENGTH]),

    KeyLength::AES128 => {
      convert_to_AES128_key!(key, key_length, key);
      // TODO: this is a mock implementation
      Ok(key)
    }
    KeyLength::AES256 => {
      convert_to_AES256_key!(key, key_length, key);
      // TODO: this is a mock implementation
      Ok(data[..16].try_into().unwrap())
    }
  }
}

// Authenticated encryption: computes the ciphertext and and a MAC for it
pub(super) fn encrypt(
  key: &Vec<u8>,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  plaintext: Vec<u8>,
) -> SecurityResult<(Vec<u8>, BuiltinMAC)> {
  // Compute the ciphertext
  let ciphertext = match key_length {
    // If no encryption is done, return the plaintext
    KeyLength::None => plaintext,

    KeyLength::AES128 => {
      convert_to_AES128_key!(key, key_length, key);
      // TODO: this is a mock implementation
      [plaintext, Vec::from(key)].concat()
    }
    KeyLength::AES256 => {
      convert_to_AES256_key!(key, key_length, key);
      // TODO: this is a mock implementation
      [plaintext, Vec::from(key)].concat()
    }
  };
  // compute the MAC for the ciphertext and return the pair
  compute_mac(key, key_length, initialization_vector, &ciphertext).map(|mac| (ciphertext, mac))
}

// Computes the MAC for the data and compares it with the one provided
pub(super) fn validate_mac(
  key: &Vec<u8>,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
  mac: BuiltinMAC,
) -> SecurityResult<()> {
  if compute_mac(key, key_length, initialization_vector, data)?.eq(&mac) {
    Ok(())
  } else {
    Err(security_error!(
      "The computed MAC does not match the provided one."
    ))
  }
}

// Authenticated decryption: validates the MAC and decrypts the ciphertext
pub(super) fn decrypt(
  key: &Vec<u8>,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  ciphertext: Vec<u8>,
  mac: BuiltinMAC,
) -> SecurityResult<Vec<u8>> {
  validate_mac(key, key_length, initialization_vector, &ciphertext, mac)?;
  match key_length {
    // If no encryption was done, the ciphertext is the plaintext
    KeyLength::None => Ok(ciphertext),

    KeyLength::AES128 => {
      convert_to_AES128_key!(key, key_length, key);
      // TODO: this is a mock implementation
      Ok(Vec::from(
        ciphertext
          .split_at(ciphertext.len() - key_length as usize)
          .0,
      ))
    }
    KeyLength::AES256 => {
      convert_to_AES256_key!(key, key_length, key);
      // TODO: this is a mock implementation
      Ok(Vec::from(
        ciphertext
          .split_at(ciphertext.len() - key_length as usize)
          .0,
      ))
    }
  }
}
