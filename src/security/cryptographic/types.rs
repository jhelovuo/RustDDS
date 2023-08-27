use serde::Serialize;
use serde::Deserialize;
use std::fmt;
use std::convert::From;

use speedy::{Readable, Writable};

use crate::{rtps::Submessage, security::types::DataHolder};
/// CryptoToken: sections 7.2.4.2 and 8.5.1.1 of the Security specification (v.
/// 1.1)
#[derive(Clone)]
pub struct CryptoToken {
  pub data_holder: DataHolder,
}
impl From<DataHolder> for CryptoToken {
  fn from(value: DataHolder) -> Self {
    CryptoToken { data_holder: value }
  }
}
pub type ParticipantCryptoToken = CryptoToken;
pub type DatawriterCryptoToken = CryptoToken;
pub type DatareaderCryptoToken = CryptoToken;

/// CryptoHandles are supposed to be opaque references to key material that can
/// only be interpreted inside the plugin implementation (8.5.1.2–4).
pub type CryptoHandle = u32;

/// ParticipantCryptoHandle: section 8.5.1.2 of the Security specification
/// (v. 1.1)
pub type ParticipantCryptoHandle = CryptoHandle;

pub type EndpointCryptoHandle = CryptoHandle;

/// DatawriterCryptoHandle: section 8.5.1.3 of the Security specification
/// (v. 1.1)
pub type DatawriterCryptoHandle = EndpointCryptoHandle;

/// DatareaderCryptoHandle: section 8.5.1.4 of the Security specification
/// (v. 1.1)
pub type DatareaderCryptoHandle = EndpointCryptoHandle;

/// CryptoTransformIdentifier: section 8.5.1.5 of the Security specification (v.
/// 1.1)
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct CryptoTransformIdentifier {
  pub transformation_kind: CryptoTransformKind,
  pub transformation_key_id: CryptoTransformKeyId,
}
/// transformation_kind: section 8.5.1.5.1 of the Security specification (v.
/// 1.1)
pub type CryptoTransformKind = [u8; 4];
/// transformation_key_id: section 8.5.1.5.2 of the Security specification (v.
/// 1.1)
#[derive(Debug, Clone, Copy, Eq, PartialEq, Readable, Writable, Serialize, Deserialize)]
pub struct CryptoTransformKeyId([u8;4]);

impl CryptoTransformKeyId {
  pub const ZERO:Self = CryptoTransformKeyId([0,0,0,0]);

  pub fn is_zero(&self) -> bool {
    *self == Self::ZERO
  } 
}

impl From<[u8;4]> for CryptoTransformKeyId {
  fn from(b:[u8;4]) -> Self {
    Self(b)
  }
}

impl fmt::Display for CryptoTransformKeyId {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "{:02x?}", self.0)
  }
}



/// SecureSubmessageCategory_t: section 8.5.1.6 of the Security specification
/// (v. 1.1)
///
/// Used as a return type by
/// [super::cryptographic_plugin::CryptoTransform::preprocess_secure_submessage],
/// and therefore includes the crypto handles that would be returned in the
/// latter two cases.

#[allow(clippy::enum_variant_names)] // We are using variant names from the spec
pub enum SecureSubmessageKind {
  InfoSubmessage,
  DatawriterSubmessage(DatawriterCryptoHandle, DatareaderCryptoHandle),
  DatareaderSubmessage(DatareaderCryptoHandle, DatawriterCryptoHandle),
}

/// [super::cryptographic_plugin::CryptoTransform::encode_datawriter_submessage]
/// and [super::cryptographic_plugin::CryptoTransform::encode_datareader_submessage]
/// may return the unencoded input or an encoded message between a
/// `SecurePrefix` and `SecurePostfix`. See 7.3.6.4.4 and 8.5.1.9.2 in DDS
/// Security v1.1.

pub enum EncodedSubmessage {
  Unencoded(Submessage),
  Encoded(Submessage, Submessage, Submessage),
}

impl From<EncodedSubmessage> for Vec<Submessage> {
  fn from(value: EncodedSubmessage) -> Self {
    match value {
      EncodedSubmessage::Unencoded(submessage) => vec![submessage],
      EncodedSubmessage::Encoded(prefix, submessage, postfix) => vec![prefix, submessage, postfix],
    }
  }
}
