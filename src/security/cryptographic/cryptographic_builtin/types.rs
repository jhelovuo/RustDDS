use byteorder::BigEndian;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::{
  messages::submessages::elements::{
    crypto_content::CryptoContent,
    crypto_footer::CryptoFooter,
    crypto_header::{CryptoHeader, PluginCryptoHeaderExtra},
  },
  security::{BinaryProperty, DataHolder, SecurityError, SecurityResult},
  security_error,
  serialization::cdr_serializer::to_bytes,
  CdrDeserializer,
};
use super::{
  CryptoHandle, CryptoToken, CryptoTransformIdentifier, CryptoTransformKeyId, CryptoTransformKind,
};

const CRYPTO_TOKEN_CLASS_ID: &str = "DDS:Crypto:AES_GCM_GMAC";
const CRYPTO_TOKEN_KEYMAT_NAME: &str = "dds.cryp.keymat";

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
        if bp0.name.eq(CRYPTO_TOKEN_KEYMAT_NAME) {
          Ok(Self {
            key_material: KeyMaterial_AES_GCM_GMAC::try_from(bp0.value.clone())?,
          })
        } else {
          Err(Self::Error {
            msg: format!(
              "The binary property of CryptoToken has the wrong name. Expected {}, got {}.",
              CRYPTO_TOKEN_KEYMAT_NAME, bp0.name
            ),
          })
        }
      }

      (CRYPTO_TOKEN_CLASS_ID, [], bps) => Err(Self::Error {
        msg: String::from(
          "CryptoToken has wrong binary_properties. Expected exactly 1 binary property.",
        ),
      }),
      (CRYPTO_TOKEN_CLASS_ID, ps, _) => Err(Self::Error {
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
          name: String::from(CRYPTO_TOKEN_KEYMAT_NAME),
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

/// KeyMaterial_AES_GCM_GMAC type from section 9.5.2.1.1 of the Security
/// specification (v. 1.1)
#[allow(non_camel_case_types)] // We use the name from the spec
#[derive(Clone)]
pub(super) struct KeyMaterial_AES_GCM_GMAC {
  pub transformation_kind: BuiltinCryptoTransformationKind,
  pub master_salt: Vec<u8>,
  pub sender_key_id: CryptoTransformKeyId,
  pub master_sender_key: BuiltinKey,
  pub receiver_specific_key_id: CryptoTransformKeyId,
  pub master_receiver_specific_key: BuiltinKey,
}

// Conversions from and into Bytes
impl TryFrom<Bytes> for KeyMaterial_AES_GCM_GMAC {
  type Error = SecurityError;
  fn try_from(value: Bytes) -> Result<Self, Self::Error> {
    // Deserialize CDR-formatted key material
    Serializable_KeyMaterial_AES_GCM_GMAC::deserialize(&mut CdrDeserializer::<
      BigEndian, /* TODO: What's the point of this constructor if we need to specify the byte
                  * order anyway */
    >::new_big_endian(value.as_ref()))
    .map_err(
      // Map deserialization error to SecurityError
      |e| Self::Error {
        msg: format!("Error deserializing KeyMaterial_AES_GCM_GMAC: {}", e),
      },
    )
    .and_then(KeyMaterial_AES_GCM_GMAC::try_from)
  }
}
impl TryFrom<KeyMaterial_AES_GCM_GMAC> for Bytes {
  type Error = SecurityError;
  fn try_from(keymat: KeyMaterial_AES_GCM_GMAC) -> Result<Self, Self::Error> {
    // Convert the key material to the serializable structure
    let serializable_keymat = Serializable_KeyMaterial_AES_GCM_GMAC::from(keymat);
    // Serialize
    to_bytes::<Serializable_KeyMaterial_AES_GCM_GMAC, BigEndian>(&serializable_keymat)
      .map(Bytes::from)
      .map_err(|e| Self::Error {
        msg: format!("Error serializing KeyMaterial_AES_GCM_GMAC: {}", e),
      })
  }
}

// Conversions from and into CryptoToken
impl TryFrom<CryptoToken> for KeyMaterial_AES_GCM_GMAC {
  type Error = SecurityError;
  fn try_from(token: CryptoToken) -> Result<Self, Self::Error> {
    BuiltinCryptoToken::try_from(token).map(KeyMaterial_AES_GCM_GMAC::from)
  }
}
impl TryFrom<KeyMaterial_AES_GCM_GMAC> for CryptoToken {
  type Error = SecurityError;
  fn try_from(key_material: KeyMaterial_AES_GCM_GMAC) -> Result<Self, Self::Error> {
    BuiltinCryptoToken::from(key_material).try_into()
  }
}

/// We need to refer to a sequence of key material structures for example in
/// register_local_datawriter. Usually the sequence has one key, but it can have
/// two if a different key is used for submessage and payload
#[allow(non_camel_case_types)] // We use the name from the spec
#[derive(Clone)]
pub(super) enum KeyMaterial_AES_GCM_GMAC_seq {
  One(KeyMaterial_AES_GCM_GMAC),
  Two(KeyMaterial_AES_GCM_GMAC, KeyMaterial_AES_GCM_GMAC),
}

impl KeyMaterial_AES_GCM_GMAC_seq {
  pub fn key(&self) -> &KeyMaterial_AES_GCM_GMAC {
    match self {
      Self::One(key) => key,
      Self::Two(key, _) => key,
    }
  }

  pub fn payload_key(&self) -> &KeyMaterial_AES_GCM_GMAC {
    match self {
      Self::One(key) => key,
      Self::Two(_, payload_key) => payload_key,
    }
  }

  pub fn modify_key<F>(self, f: F) -> KeyMaterial_AES_GCM_GMAC_seq
  where
    F: FnOnce(KeyMaterial_AES_GCM_GMAC) -> KeyMaterial_AES_GCM_GMAC,
  {
    match self {
      Self::One(key) => Self::One(f(key)),
      Self::Two(key, payload_key) => Self::Two(f(key), payload_key),
    }
  }

  pub fn add_receiver_specific_key(
    self,
    receiver_specific_key_id: CryptoTransformKeyId,
    master_receiver_specific_key: BuiltinKey,
  ) -> KeyMaterial_AES_GCM_GMAC_seq {
    self.modify_key(
      |KeyMaterial_AES_GCM_GMAC {
         transformation_kind,
         master_salt,
         master_sender_key,
         sender_key_id,
         ..
       }| KeyMaterial_AES_GCM_GMAC {
        transformation_kind,
        master_salt,
        master_sender_key,
        sender_key_id,
        receiver_specific_key_id,
        master_receiver_specific_key,
      },
    )
  }
}

impl TryFrom<Vec<KeyMaterial_AES_GCM_GMAC>> for KeyMaterial_AES_GCM_GMAC_seq {
  type Error = SecurityError;
  fn try_from(value: Vec<KeyMaterial_AES_GCM_GMAC>) -> Result<Self, Self::Error> {
    match value.as_slice() {
      [key] => Ok(KeyMaterial_AES_GCM_GMAC_seq::One(key.clone())),
      [key, payload_key] => Ok(KeyMaterial_AES_GCM_GMAC_seq::Two(
        key.clone(),
        payload_key.clone(),
      )),
      [] => Ok(KeyMaterial_AES_GCM_GMAC_seq::One(
        KeyMaterial_AES_GCM_GMAC {
          transformation_kind: BuiltinCryptoTransformationKind::CRYPTO_TRANSFORMATION_KIND_NONE,
          master_salt: Vec::new(),
          sender_key_id: 0,
          master_sender_key: Vec::new(),
          receiver_specific_key_id: 0,
          master_receiver_specific_key: Vec::new(),
        },
      )),
      _ => Err(security_error!(
        "Expected 1 or 2 key materials in KeyMaterial_AES_GCM_GMAC_seq, received {}",
        value.len()
      )),
    }
  }
}
impl From<KeyMaterial_AES_GCM_GMAC_seq> for Vec<KeyMaterial_AES_GCM_GMAC> {
  fn from(value: KeyMaterial_AES_GCM_GMAC_seq) -> Self {
    match value {
      KeyMaterial_AES_GCM_GMAC_seq::One(key) => vec![key],
      KeyMaterial_AES_GCM_GMAC_seq::Two(key, payload_key) => vec![key, payload_key],
    }
  }
}

// Conversions from and into Bytes for KeyMaterial_AES_GCM_GMAC_seq
impl TryFrom<Bytes> for KeyMaterial_AES_GCM_GMAC_seq {
  type Error = SecurityError;
  fn try_from(value: Bytes) -> Result<Self, Self::Error> {
    // Deserialize CDR-formatted key material
    let serializable_keymat_seq =
      Vec::<Serializable_KeyMaterial_AES_GCM_GMAC>::deserialize(&mut CdrDeserializer::<
        BigEndian, /* TODO: What's the point of this constructor if we need to specify the byte
                    * order anyway */
      >::new_big_endian(
        value.as_ref()
      ))
      .map_err(
        // Map deserialization error to SecurityError
        |e| Self::Error {
          msg: format!("Error deserializing Vec<KeyMaterial_AES_GCM_GMAC>: {}", e),
        },
      )?;

    serializable_keymat_seq
      // Map transformation_kind to builtin for each keymat
      .iter()
      .map(|serializable_keymat| KeyMaterial_AES_GCM_GMAC::try_from(serializable_keymat.clone()))
      // Convert to Vec and dig out the Result
      .collect::<Result<Vec<KeyMaterial_AES_GCM_GMAC>, Self::Error>>()
      // Convert the Vec
      .and_then(KeyMaterial_AES_GCM_GMAC_seq::try_from)
  }
}

impl TryFrom<KeyMaterial_AES_GCM_GMAC_seq> for Bytes {
  type Error = SecurityError;
  fn try_from(keymat_seq: KeyMaterial_AES_GCM_GMAC_seq) -> Result<Self, Self::Error> {
    // Convert the key material to the serializable structure
    let serializable_keymat_seq = Vec::from(keymat_seq)
      .iter()
      .map(|keymat| Serializable_KeyMaterial_AES_GCM_GMAC::from(keymat.clone()))
      .collect();

    // Serialize
    to_bytes::<Vec<Serializable_KeyMaterial_AES_GCM_GMAC>, BigEndian>(&serializable_keymat_seq)
      .map(Bytes::from)
      .map_err(|e| Self::Error {
        msg: format!("Error serializing KeyMaterial_AES_GCM_GMAC_seq: {}", e),
      })
  }
}

impl KeyMaterial_AES_GCM_GMAC {
  /// Checks that the key material matches the given common key material and
  /// returns the receiver-specific material
  pub fn receiver_key_for(
    &self,
    KeyMaterial_AES_GCM_GMAC {
      transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      ..
    }: &KeyMaterial_AES_GCM_GMAC,
  ) -> SecurityResult<ReceiverKeyMaterial> {
    if !self.sender_key_id.eq(sender_key_id) {
      Err(security_error!(
        "The receiver-specific key has a wrong sender_key_id: expected {:?}, received {:?}.",
        sender_key_id,
        self.sender_key_id
      ))
    } else if !self.transformation_kind.eq(transformation_kind) {
      Err(security_error!(
        "The receiver-specific key has a wrong transformation_kind: expected {:?}, received {:?}.",
        transformation_kind,
        self.transformation_kind
      ))
    } else if !self.master_sender_key.eq(master_sender_key) {
      Err(security_error!(
        "The receiver-specific key has a wrong master_sender_key: expected {:?}, received {:?}.",
        master_sender_key,
        self.master_sender_key
      ))
    } else if !self.master_salt.eq(master_salt) {
      Err(security_error!(
        "The receiver-specific key has a wrong master_salt: expected {:?}, received {:?}.",
        master_salt,
        self.master_salt
      ))
    } else {
      Ok(ReceiverKeyMaterial {
        receiver_specific_key_id: self.receiver_specific_key_id,
        master_receiver_specific_key: self.master_receiver_specific_key.clone(),
      })
    }
  }
}

pub(super) struct ReceiverKeyMaterial {
  pub receiver_specific_key_id: CryptoTransformKeyId,
  pub master_receiver_specific_key: BuiltinKey,
}

// Conversions from and into Vec<CryptoToken> for KeyMaterial_AES_GCM_GMAC_seq
impl TryFrom<Vec<CryptoToken>> for KeyMaterial_AES_GCM_GMAC_seq {
  type Error = SecurityError;
  fn try_from(tokens: Vec<CryptoToken>) -> Result<Self, Self::Error> {
    tokens
      .iter()
      .map(|token| KeyMaterial_AES_GCM_GMAC::try_from(token.clone()))
      .collect::<Result<Vec<KeyMaterial_AES_GCM_GMAC>, Self::Error>>()
      // Convert the Vec
      .and_then(KeyMaterial_AES_GCM_GMAC_seq::try_from)
  }
}
impl TryFrom<KeyMaterial_AES_GCM_GMAC_seq> for Vec<CryptoToken> {
  type Error = SecurityError;
  fn try_from(keymat_seq: KeyMaterial_AES_GCM_GMAC_seq) -> Result<Self, Self::Error> {
    Vec::from(keymat_seq)
      .iter()
      .map(|keymat| CryptoToken::try_from(keymat.clone()))
      .collect()
  }
}
//For (de)serialization
#[allow(non_camel_case_types)] // We use the name from the spec
#[derive(Deserialize, Serialize, PartialEq, Clone)]
struct Serializable_KeyMaterial_AES_GCM_GMAC {
  transformation_kind: CryptoTransformKind,
  master_salt: Vec<u8>,
  sender_key_id: CryptoTransformKeyId,
  master_sender_key: BuiltinKey,
  receiver_specific_key_id: CryptoTransformKeyId,
  master_receiver_specific_key: BuiltinKey,
}
impl TryFrom<Serializable_KeyMaterial_AES_GCM_GMAC> for KeyMaterial_AES_GCM_GMAC {
  type Error = SecurityError;
  fn try_from(
    Serializable_KeyMaterial_AES_GCM_GMAC {
      transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      receiver_specific_key_id,
      master_receiver_specific_key,
    }: Serializable_KeyMaterial_AES_GCM_GMAC,
  ) -> Result<Self, Self::Error> {
    // Map transformation_kind to builtin
    BuiltinCryptoTransformationKind::try_from(transformation_kind)
      // Construct a keymat
      .map(|transformation_kind| Self {
        transformation_kind,
        master_salt,
        sender_key_id,
        master_sender_key,
        receiver_specific_key_id,
        master_receiver_specific_key,
      })
  }
}
impl From<KeyMaterial_AES_GCM_GMAC> for Serializable_KeyMaterial_AES_GCM_GMAC {
  fn from(
    KeyMaterial_AES_GCM_GMAC {
      transformation_kind,
      master_salt,
      sender_key_id,
      master_sender_key,
      receiver_specific_key_id,
      master_receiver_specific_key,
    }: KeyMaterial_AES_GCM_GMAC,
  ) -> Self {
    Serializable_KeyMaterial_AES_GCM_GMAC {
      // Serialize transformation_kind
      transformation_kind: transformation_kind.into(),
      master_salt,
      sender_key_id,
      master_sender_key,
      receiver_specific_key_id,
      master_receiver_specific_key,
    }
  }
}

/// Valid values for CryptoTransformKind from section 9.5.2.1.1 of the Security
/// specification (v. 1.1)
#[allow(non_camel_case_types)] // We use the names from the spec
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum BuiltinCryptoTransformationKind {
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
pub(super) type BuiltinInitializationVector = [u8; INITIALIZATION_VECTOR_LENGTH];
pub(super) struct BuiltinCryptoHeaderExtra(pub(super) BuiltinInitializationVector);

/// Methods for getting the contained data
impl BuiltinCryptoHeaderExtra {
  pub(super) fn initialization_vector(&self) -> BuiltinInitializationVector {
    self.0
  }
  pub(super) fn session_id(&self) -> [u8; 4] {
    // Succeeds as the slice length is 4
    <[u8; 4]>::try_from(&self.0[..4]).unwrap()
  }
  pub(super) fn initialization_vector_suffix(&self) -> [u8; 8] {
    // Succeeds as the slice length is 12-4=8
    <[u8; 8]>::try_from(&self.0[4..]).unwrap()
  }
}

impl From<BuiltinInitializationVector> for BuiltinCryptoHeaderExtra {
  fn from(value: BuiltinInitializationVector) -> Self {
    Self(value)
  }
}
// Conversion from session_id and initialization_vector_suffix
impl From<([u8; 4], [u8; 8])> for BuiltinCryptoHeaderExtra {
  fn from((session_id, initialization_vector_suffix): ([u8; 4], [u8; 8])) -> Self {
    Self::from(
      // Succeeds as the vector length is 4+8=12
      BuiltinInitializationVector::try_from(
        [
          Vec::from(session_id),
          Vec::from(initialization_vector_suffix),
        ]
        .concat(),
      )
      .unwrap(),
    )
  }
}
impl From<BuiltinCryptoHeaderExtra> for PluginCryptoHeaderExtra {
  fn from(value: BuiltinCryptoHeaderExtra) -> Self {
    Self::from(Vec::from(value.initialization_vector()))
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
    BuiltinInitializationVector::try_from(data)
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
pub(super) struct BuiltinCryptoHeader {
  pub transform_identifier: BuiltinCryptoTransformIdentifier,
  pub builtin_crypto_header_extra: BuiltinCryptoHeaderExtra,
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
#[derive(Deserialize, Serialize, PartialEq)]
pub(super) struct BuiltinCryptoFooter {
  pub common_mac: BuiltinMAC,
  pub receiver_specific_macs: Vec<ReceiverSpecificMAC>,
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
#[derive(Deserialize, Serialize, PartialEq)]
pub(super) struct ReceiverSpecificMAC {
  pub receiver_mac_key_id: CryptoTransformKeyId,
  pub receiver_mac: BuiltinMAC,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub(super) enum EntityCategory {
  DataReader,
  DataWriter,
}
impl EntityCategory {
  pub fn opposite(self) -> EntityCategory {
    match self {
      EntityCategory::DataReader => EntityCategory::DataWriter,
      EntityCategory::DataWriter => EntityCategory::DataReader,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub(super) struct EntityInfo {
  pub handle: CryptoHandle,
  pub category: EntityCategory,
}

pub(super) const AES128_KEY_LENGTH: usize = 16;
pub(super) type AES128Key = [u8; AES128_KEY_LENGTH];
pub(super) const AES256_KEY_LENGTH: usize = 32;
pub(super) type AES256Key = [u8; AES256_KEY_LENGTH];

#[derive(Debug, Clone, Copy)]
pub(super) enum KeyLength {
  None = 0, // Mostly for testing and debugging
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

// The keys are given as byte sequences that can be empty or of length 16 or 32
pub(super) type BuiltinKey = Vec<u8>;
