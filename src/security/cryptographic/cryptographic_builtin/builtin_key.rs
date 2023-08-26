use crate::security::{security_error, SecurityError, SecurityResult};
use super::types::BuiltinCryptoTransformationKind;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum BuiltinKey {
  AES128([u8; AES128_KEY_LENGTH]),
  AES256([u8; AES256_KEY_LENGTH]),
}

impl BuiltinKey {
  pub const ZERO: Self = BuiltinKey::AES256([0; AES256_KEY_LENGTH]);

  pub fn as_bytes(&self) -> &[u8] {
    match self {
      BuiltinKey::AES128(d) => d,
      BuiltinKey::AES256(d) => d,
    }
  }

  pub fn from_bytes(length: KeyLength, bytes: &[u8]) -> SecurityResult<Self> {
    let l = length as usize;
    match length {
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

  pub fn len(&self) -> usize {
    match self {
      BuiltinKey::AES128(d) => d.len(),
      BuiltinKey::AES256(d) => d.len(),
    }
  }

  pub fn key_length(&self) -> KeyLength {
    match self {
      BuiltinKey::AES128(_) => KeyLength::AES128,
      BuiltinKey::AES256(_) => KeyLength::AES256,
    }
  }


  // Rust `rand` library uses by default the 12-round chacha-algorithm, which is
  // "widely believed" to be secure.
  // The library documentation states that the generator may be upgraded, if it is
  // found to be insecure.
  pub fn generate_random(key_len: KeyLength) -> Self {
    match key_len {
      KeyLength::AES128 => BuiltinKey::AES128(rand::random::<[u8; AES128_KEY_LENGTH]>()),
      KeyLength::AES256 => BuiltinKey::AES256(rand::random::<[u8; AES256_KEY_LENGTH]>()),
    }
  }
}

pub(crate) const AES128_KEY_LENGTH: usize = 16;
pub(crate) type AES128Key = [u8; AES128_KEY_LENGTH];

pub(crate) const AES256_KEY_LENGTH: usize = 32;
pub(crate) type AES256Key = [u8; AES256_KEY_LENGTH];

#[derive(Debug, Clone, Copy)]
pub(crate) enum KeyLength {
  //None = 0, // for cases where encryption or signing is not requested
  AES128 = AES128_KEY_LENGTH as isize,
  AES256 = AES256_KEY_LENGTH as isize,
}

impl TryFrom<BuiltinCryptoTransformationKind> for KeyLength {
  type Error = SecurityError;

  fn try_from(value: BuiltinCryptoTransformationKind) -> Result<Self, SecurityError> {
    match value {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => Err(security_error(
        "No crypto transform requested, no key length",
      )),
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC => Ok(Self::AES128),
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM => Ok(Self::AES128),
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => Ok(Self::AES256),
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => Ok(Self::AES256),
    }
  }
}
