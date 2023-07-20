use crate::{
  security::{SecurityError, SecurityResult},
  security_error,
};
use super::types::{
  AES128Key, AES256Key, BuiltinInitializationVector, BuiltinMAC, KeyLength, MAC_LENGTH,
};

#[doc(hidden)]
#[macro_export]
macro_rules! key_length_error {
  ($l:expr,$key_length:expr) => {
    |_| security_error!("Got a {} bytes long key, expected {:?}.", $l, $key_length)
  };
}

#[doc(hidden)]
#[macro_export]
macro_rules! convert_to_AES128_key {
  ($key:expr,$key_length:expr,$variable_name:ident) => {
    let $variable_name =
      AES128Key::try_from($key.clone()).map_err(key_length_error!($key.len(), $key_length))?;
  };
}
#[doc(hidden)]
#[macro_export]
macro_rules! convert_to_AES256_key {
  ($key:expr,$key_length:expr,$variable_name:ident) => {
    let $variable_name =
      AES256Key::try_from($key.clone()).map_err(key_length_error!($key.len(), $key_length))?;
  };
}

pub(super) fn compute_mac(
  key: &Vec<u8>,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  data: &[u8],
) -> SecurityResult<BuiltinMAC> {
  match key_length {
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

pub(super) fn encrypt(
  key: &Vec<u8>,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  plaintext: Vec<u8>,
) -> SecurityResult<(Vec<u8>, BuiltinMAC)> {
  let ciphertext = match key_length {
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
  compute_mac(key, key_length, initialization_vector, &ciphertext).map(|mac| (ciphertext, mac))
}

pub(super) fn validate_mac(
  key: &Vec<u8>,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  plaintext: &[u8],
  mac: BuiltinMAC,
) -> SecurityResult<()> {
  if compute_mac(key, key_length, initialization_vector, plaintext)?.eq(&mac) {
    Ok(())
  } else {
    Err(security_error!(
      "The computed MAC does not match the provided one."
    ))
  }
}

pub(super) fn decrypt(
  key: &Vec<u8>,
  key_length: KeyLength,
  initialization_vector: BuiltinInitializationVector,
  ciphertext: Vec<u8>,
  mac: BuiltinMAC,
) -> SecurityResult<Vec<u8>> {
  validate_mac(key, key_length, initialization_vector, &ciphertext, mac)?;
  match key_length {
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
