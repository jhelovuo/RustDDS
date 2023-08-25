use byteorder::BigEndian;
use serde::{Deserialize, Serialize};
use speedy::Readable;

use crate::{
  messages::submessages::elements::{
    crypto_content::CryptoContent,
    crypto_footer::CryptoFooter,
    crypto_header::{CryptoHeader, PluginCryptoHeaderExtra},
  },
  security::{cryptographic::EndpointCryptoHandle, BinaryProperty, DataHolder, SecurityError},
  security_error,
  serialization::cdr_serializer::to_bytes,
  CdrDeserializer,
};
use super::{
  key_material::*, CryptoToken, CryptoTransformIdentifier, CryptoTransformKeyId,
  CryptoTransformKind,
};

const CRYPTO_TOKEN_CLASS_ID: &str = "DDS:Crypto:AES_GCM_GMAC";
const CRYPTO_TOKEN_KEY_MATERIAL_NAME: &str = "dds.cryp.keymat";

/// DDS:Crypto:AES-GCM-GMAC CryptoToken type from section 9.5.2.1 of the
/// Security specification (v. 1.1)
pub(super) struct BuiltinCryptoToken {
  pub key_material: KeyMaterial_AES_GCM_GMAC,
}
impl TryFrom<CryptoToken> for BuiltinCryptoToken {
  type Error = SecurityError;
  fn try_from(value: CryptoToken) -> Result<Self, Self::Error> {
    let dh = value.data_holder;
    match (
      dh.class_id.as_str(),
      dh.properties.as_slice(),
      dh.binary_properties.as_slice(),
    ) {
      (CRYPTO_TOKEN_CLASS_ID, [], [bp0]) => {
        if bp0.name.eq(CRYPTO_TOKEN_KEY_MATERIAL_NAME) {
          Ok(Self {
            key_material: KeyMaterial_AES_GCM_GMAC::try_from(bp0.value.clone())?,
          })
        } else {
          Err(Self::Error {
            msg: format!(
              "The binary property of CryptoToken has the wrong name. Expected {}, got {}.",
              CRYPTO_TOKEN_KEY_MATERIAL_NAME, bp0.name
            ),
          })
        }
      }

      (CRYPTO_TOKEN_CLASS_ID, [], _) => Err(Self::Error {
        msg: String::from(
          "CryptoToken has wrong binary_properties. Expected exactly 1 binary property.",
        ),
      }),
      (CRYPTO_TOKEN_CLASS_ID, _, _) => Err(Self::Error {
        msg: String::from("CryptoToken has wrong properties. Expected properties to be empty."),
      }),

      (cid, _, _) => Err(Self::Error {
        msg: format!(
          "CryptoToken has wrong class_id. Expected {}, got {}",
          CRYPTO_TOKEN_CLASS_ID, cid
        ),
      }),
    }
  }
}

impl TryFrom<BuiltinCryptoToken> for CryptoToken {
  type Error = SecurityError;
  fn try_from(value: BuiltinCryptoToken) -> Result<Self, Self::Error> {
    Ok(CryptoToken {
      data_holder: DataHolder {
        class_id: String::from(CRYPTO_TOKEN_CLASS_ID),
        properties: Vec::new(),
        binary_properties: Vec::from([BinaryProperty {
          name: String::from(CRYPTO_TOKEN_KEY_MATERIAL_NAME),
          value: value.key_material.try_into()?,
          propagate: true,
        }]),
      },
    })
  }
}

impl From<KeyMaterial_AES_GCM_GMAC> for BuiltinCryptoToken {
  fn from(key_material: KeyMaterial_AES_GCM_GMAC) -> Self {
    Self { key_material }
  }
}
impl From<BuiltinCryptoToken> for KeyMaterial_AES_GCM_GMAC {
  fn from(BuiltinCryptoToken { key_material }: BuiltinCryptoToken) -> Self {
    key_material
  }
}

/// Valid values for CryptoTransformKind from section 9.5.2.1.1 of the Security
/// specification (v. 1.1)
#[allow(non_camel_case_types)] // We use the names from the spec
#[derive(Copy, Clone, PartialEq, Eq, Debug, Readable)]
pub(crate) enum BuiltinCryptoTransformationKind {
  CRYPTO_TRANSFORMATION_KIND_NONE,
  CRYPTO_TRANSFORMATION_KIND_AES128_GMAC,
  CRYPTO_TRANSFORMATION_KIND_AES128_GCM,
  CRYPTO_TRANSFORMATION_KIND_AES256_GMAC,
  CRYPTO_TRANSFORMATION_KIND_AES256_GCM,
}
impl TryFrom<CryptoTransformKind> for BuiltinCryptoTransformationKind {
  type Error = SecurityError;
  fn try_from(value: CryptoTransformKind) -> Result<Self, Self::Error> {
    match value {
      [0, 0, 0, 0] => Ok(Self::CRYPTO_TRANSFORMATION_KIND_NONE),
      [0, 0, 0, 1] => Ok(Self::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC),
      [0, 0, 0, 2] => Ok(Self::CRYPTO_TRANSFORMATION_KIND_AES128_GCM),
      [0, 0, 0, 3] => Ok(Self::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC),
      [0, 0, 0, 4] => Ok(Self::CRYPTO_TRANSFORMATION_KIND_AES256_GCM),
      _ => Err(Self::Error {
        msg: String::from("Invalid CryptoTransformKind"),
      }),
    }
  }
}
impl From<BuiltinCryptoTransformationKind> for CryptoTransformKind {
  fn from(builtin: BuiltinCryptoTransformationKind) -> CryptoTransformKind {
    match builtin {
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE => [0, 0, 0, 0],
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GMAC => [0, 0, 0, 1],
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES128_GCM => [0, 0, 0, 2],
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GMAC => [0, 0, 0, 3],
      BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_AES256_GCM => [0, 0, 0, 4],
    }
  }
}

/// CryptoTransformIdentifier type from section 9.5.2.2 of the Security
/// specification (v. 1.1)
#[derive(Readable)]
pub(super) struct BuiltinCryptoTransformIdentifier {
  pub transformation_kind: BuiltinCryptoTransformationKind,
  pub transformation_key_id: CryptoTransformKeyId,
}
impl TryFrom<CryptoTransformIdentifier> for BuiltinCryptoTransformIdentifier {
  type Error = SecurityError;
  fn try_from(value: CryptoTransformIdentifier) -> Result<Self, Self::Error> {
    match BuiltinCryptoTransformationKind::try_from(value.transformation_kind) {
      Err(e) => Err(e),
      Ok(transformation_kind) => Ok(Self {
        transformation_kind,
        transformation_key_id: value.transformation_key_id,
      }),
    }
  }
}
impl From<BuiltinCryptoTransformIdentifier> for CryptoTransformIdentifier {
  fn from(
    BuiltinCryptoTransformIdentifier {
      transformation_kind,
      transformation_key_id,
    }: BuiltinCryptoTransformIdentifier,
  ) -> Self {
    CryptoTransformIdentifier {
      transformation_kind: transformation_kind.into(),
      transformation_key_id,
    }
  }
}

/// The plugin_crypto_header_extra contains the initialization vector, which
/// consists of the session_id and initialization_vector_suffix. 9.5.2.3
pub(super) const INITIALIZATION_VECTOR_LENGTH: usize = 12;

#[derive(Debug, Clone, Copy, Readable)]
pub(super) struct BuiltinInitializationVector([u8; INITIALIZATION_VECTOR_LENGTH]);

impl BuiltinInitializationVector {
  pub(super) fn new(session_id: SessionId, initialization_vector_suffix: [u8; 8]) -> Self {
    BuiltinInitializationVector(
      // Concatenate
      [
        Vec::from(session_id.0),
        Vec::from(initialization_vector_suffix),
      ]
      .concat()
      .try_into()
      .unwrap(), // 4+8=12
    )
  }
  pub(super) fn session_id(&self) -> SessionId {
    // Succeeds as the slice length is 4
    SessionId::new(<[u8; 4]>::try_from(&self.0[..4]).unwrap())
  }
  pub(super) fn initialization_vector_suffix(&self) -> [u8; 8] {
    // Succeeds as the slice length is 12-4=8
    <[u8; 8]>::try_from(&self.0[4..]).unwrap()
  }

  pub fn try_from_slice(s: impl AsRef<[u8]>) -> Result<Self, std::array::TryFromSliceError> {
    Ok(Self(<[u8; INITIALIZATION_VECTOR_LENGTH]>::try_from(
      s.as_ref(),
    )?))
  }
}

impl From<BuiltinInitializationVector> for [u8; INITIALIZATION_VECTOR_LENGTH] {
  fn from(value: BuiltinInitializationVector) -> [u8; INITIALIZATION_VECTOR_LENGTH] {
    value.0
  }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SessionId([u8; 4]);

impl SessionId {
  pub fn new(s: [u8; 4]) -> Self {
    SessionId(s)
  }

  pub fn as_bytes(&self) -> &[u8] {
    &self.0
  }
}

#[derive(Debug, Clone, Copy, Readable)]
pub(crate) struct BuiltinCryptoHeaderExtra(pub(super) BuiltinInitializationVector);

/// Methods for getting the contained data
impl BuiltinCryptoHeaderExtra {
  pub(super) fn initialization_vector(&self) -> BuiltinInitializationVector {
    self.0
  }
  pub(super) fn session_id(&self) -> SessionId {
    self.0.session_id()
  }
  pub(super) fn initialization_vector_suffix(&self) -> [u8; 8] {
    self.0.initialization_vector_suffix()
  }
  pub fn new(session_id: SessionId, initialization_vector_suffix: [u8; 8]) -> Self {
    Self::from((session_id, initialization_vector_suffix))
  }

  pub fn serialized_len() -> usize {
    INITIALIZATION_VECTOR_LENGTH
  }
}

impl From<BuiltinInitializationVector> for BuiltinCryptoHeaderExtra {
  fn from(value: BuiltinInitializationVector) -> Self {
    Self(value)
  }
}

// Conversion from session_id and initialization_vector_suffix
impl From<(SessionId, [u8; 8])> for BuiltinCryptoHeaderExtra {
  fn from((session_id, initialization_vector_suffix): (SessionId, [u8; 8])) -> Self {
    Self(BuiltinInitializationVector::new(
      session_id,
      initialization_vector_suffix,
    ))
  }
}

impl From<BuiltinCryptoHeaderExtra> for PluginCryptoHeaderExtra {
  fn from(value: BuiltinCryptoHeaderExtra) -> Self {
    Self::from(Vec::from(value.initialization_vector().0))
  }
}

impl TryFrom<PluginCryptoHeaderExtra> for BuiltinCryptoHeaderExtra {
  type Error = SecurityError;
  fn try_from(
    PluginCryptoHeaderExtra { data }: PluginCryptoHeaderExtra,
  ) -> Result<Self, Self::Error> {
    // Save the length for the error message
    let plugin_crypto_header_length = data.len();
    // Convert to fixed-length array
    BuiltinInitializationVector::try_from_slice(data)
      .map_err(|_| {
        security_error!(
          "plugin_crypto_header_extra was of length {}. Expected {}.",
          plugin_crypto_header_length,
          INITIALIZATION_VECTOR_LENGTH
        )
      })
      // Wrap the initialization vector
      .map(Self::from)
  }
}

/// CryptoHeader type from section 9.5.2.3 of the Security specification (v.
/// 1.1)
#[derive(Readable)]
pub(super) struct BuiltinCryptoHeader {
  pub transform_identifier: BuiltinCryptoTransformIdentifier, // 4+4 bytes
  pub builtin_crypto_header_extra: BuiltinCryptoHeaderExtra,  // 4+8 bytes
}

impl BuiltinCryptoHeader {
  pub fn serialized_len() -> usize {
    4 + 4 + 4 + 8
  }
}

impl TryFrom<CryptoHeader> for BuiltinCryptoHeader {
  type Error = SecurityError;
  fn try_from(
    CryptoHeader {
      transformation_id,
      plugin_crypto_header_extra,
    }: CryptoHeader,
  ) -> Result<Self, Self::Error> {
    // Try to cast [CryptoTransformIdentifier] to [BuiltinCryptoTransformIdentifier]
    // and read the initialization vector from 'crypto_header_extra'
    Ok(Self {
      transform_identifier: BuiltinCryptoTransformIdentifier::try_from(transformation_id)?,
      builtin_crypto_header_extra: BuiltinCryptoHeaderExtra::try_from(plugin_crypto_header_extra)?,
    })
  }
}

impl From<BuiltinCryptoHeader> for CryptoHeader {
  fn from(
    BuiltinCryptoHeader {
      transform_identifier,
      builtin_crypto_header_extra,
    }: BuiltinCryptoHeader,
  ) -> Self {
    CryptoHeader {
      transformation_id: transform_identifier.into(),
      plugin_crypto_header_extra: builtin_crypto_header_extra.into(),
    }
  }
}

/// CryptoContent type from section 9.5.2.4 of the Security specification (v.
/// 1.1)
pub(super) type BuiltinCryptoContent = CryptoContent;

pub(super) const MAC_LENGTH: usize = 16;
pub(super) type BuiltinMAC = [u8; MAC_LENGTH];

/// CryptoFooter type from section 9.5.2.5 of the Security specification (v.
/// 1.1)
#[derive(Deserialize, Serialize, PartialEq, Readable)]
pub(super) struct BuiltinCryptoFooter {
  pub common_mac: BuiltinMAC,
  pub receiver_specific_macs: Vec<ReceiverSpecificMAC>,
}

impl BuiltinCryptoFooter {
  pub fn minimal_serialized_len() -> usize {
    MAC_LENGTH  // common_mac
    + 4 // receiver_specific_macs = Vec::new()
  }

  pub fn only_common_mac(common_mac: BuiltinMAC) -> Self {
    BuiltinCryptoFooter { common_mac, receiver_specific_macs: Vec::new() }
  }
}

impl TryFrom<Vec<u8>> for BuiltinCryptoFooter {
  type Error = SecurityError;
  fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
    // Deserialize the data
    BuiltinCryptoFooter::deserialize(&mut CdrDeserializer::<
      BigEndian, /* TODO: What's the point of this constructor if we need to specify the byte
                  * order anyway */
    >::new_big_endian(data.as_ref()))
    .map_err(
      // Map deserialization error to SecurityError
      |e| Self::Error {
        msg: format!("Error deserializing BuiltinCryptoFooter: {}", e),
      },
    )
  }
}
impl TryFrom<CryptoFooter> for BuiltinCryptoFooter {
  type Error = SecurityError;
  fn try_from(value: CryptoFooter) -> Result<Self, Self::Error> {
    <Vec<u8>>::from(value).try_into()
  }
}
impl TryFrom<BuiltinCryptoFooter> for Vec<u8> {
  type Error = SecurityError;
  fn try_from(value: BuiltinCryptoFooter) -> Result<Self, Self::Error> {
    // Serialize
    to_bytes::<BuiltinCryptoFooter, BigEndian>(&value).map_err(|e| Self::Error {
      msg: format!("Error serializing BuiltinCryptoFooter: {}", e),
    })
  }
}
impl TryFrom<BuiltinCryptoFooter> for CryptoFooter {
  type Error = SecurityError;
  fn try_from(value: BuiltinCryptoFooter) -> Result<Self, Self::Error> {
    <Vec<u8>>::try_from(value).map(Self::from)
  }
}

/// ReceiverSpecificMAC type from section 9.5.2.5 of the Security specification
/// (v. 1.1)
#[derive(Deserialize, Serialize, PartialEq, Readable)]
pub(super) struct ReceiverSpecificMAC {
  pub receiver_mac_key_id: CryptoTransformKeyId,
  pub receiver_mac: BuiltinMAC,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub(super) enum EndpointKind {
  DataReader,
  DataWriter,
}
impl EndpointKind {
  pub fn opposite(self) -> EndpointKind {
    match self {
      EndpointKind::DataReader => EndpointKind::DataWriter,
      EndpointKind::DataWriter => EndpointKind::DataReader,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub(super) struct EndpointInfo {
  pub crypto_handle: EndpointCryptoHandle,
  pub kind: EndpointKind,
}
