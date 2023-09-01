use crate::security::{security_error, SecurityResult};
use super::types::BuiltinCryptoTransformationKind;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) enum BuiltinKey {
  None,
  AES128([u8; AES128_KEY_LENGTH]),
  AES256([u8; AES256_KEY_LENGTH]),
}

impl BuiltinKey {
  pub(super) fn as_bytes(&self) -> &[u8] {
    match self {
      BuiltinKey::None => &[],
      BuiltinKey::AES128(d) => d,
      BuiltinKey::AES256(d) => d,
    }
  }

  pub(super) fn from_bytes(length: KeyLength, bytes: &[u8]) -> SecurityResult<Self> {
    let l = length as usize;
    match length {
      KeyLength::None => Ok(BuiltinKey::None),

      // unwraps will succeed, because we test for sufficient length just before.
      KeyLength::AES128 if bytes.len() >= l => {
        Ok(BuiltinKey::AES128(bytes[0..l].try_into().unwrap()))
      }

      KeyLength::AES256 if bytes.len() >= l => {
        Ok(BuiltinKey::AES256(bytes[0..l].try_into().unwrap()))
      }

      _ => Err(security_error("BuiltinKey: source material too short.")),
    }
  }

  pub(super) fn len(&self) -> usize {
    match self {
      BuiltinKey::None => 0,
      BuiltinKey::AES128(d) => d.len(),
      BuiltinKey::AES256(d) => d.len(),
    }
  }

  pub(super) fn key_length(&self) -> KeyLength {
    match self {
      BuiltinKey::None => KeyLength::None,
      BuiltinKey::AES128(_) => KeyLength::AES128,
      BuiltinKey::AES256(_) => KeyLength::AES256,
    }
  }

  // Rust `rand` library uses by default the 12-round chacha-algorithm, which is
  // "widely believed" to be secure.
  // The library documentation states that the generator may be upgraded, if it is
  // found to be insecure.
  pub(super) fn generate_random(key_len: KeyLength) -> Self {
    match key_len {
      KeyLength::None => BuiltinKey::None,
      KeyLength::AES128 => BuiltinKey::AES128(rand::random::<[u8; AES128_KEY_LENGTH]>()),
      KeyLength::AES256 => BuiltinKey::AES256(rand::random::<[u8; AES256_KEY_LENGTH]>()),
    }
  }
}

pub(super) const AES128_KEY_LENGTH: usize = 16;
pub(super) type AES128Key = [u8; AES128_KEY_LENGTH];

pub(super) const AES256_KEY_LENGTH: usize = 32;
pub(super) type AES256Key = [u8; AES256_KEY_LENGTH];

#[derive(Debug, Clone, Copy)]
pub(super) enum KeyLength {
  None = 0, // for cases where encryption or signing is not requested
  AES128 = AES128_KEY_LENGTH as isize,
  AES256 = AES256_KEY_LENGTH as isize,
}

impl From<BuiltinCryptoTransformationKind> for KeyLength {
  fn from(value: BuiltinCryptoTransformationKind) -> Self {
    match value {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => Self::None,
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC => Self::AES128,
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM => Self::AES128,
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => Self::AES256,
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => Self::AES256,
    }
  }
}
