use bytes::Bytes;
use log::debug;

use crate::{
  security::{authentication::types::*, DataHolderBuilder, SecurityError},
  security_error,
};

pub const IDENTITY_TOKEN_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0";

// Expected property names in IdentityToken
pub(in crate::security) const CERT_SN_PROPERTY_NAME: &str = "dds.cert.sn";
const CERT_ALGO_PROPERTY_NAME: &str = "dds.cert.algo";
const CA_SN_PROPERTY_NAME: &str = "dds.ca.sn";
const CA_ALGO_PROPERTY_NAME: &str = "dds.ca.algo";

// Accepted values for the algorithm properties in IdentityToken
const RSA_2048_ALGO_NAME: &str = "RSA-2048";
pub(in crate::security) const RSA_2048_KEY_LENGTH: usize = 256;
const EC_PRIME_ALGO_NAME: &str = "EC-prime256v1";

#[derive(Debug, Clone, Copy)]
pub(in crate::security) enum CertificateAlgorithm {
  RSA2048,
  ECPrime256v1,
}
impl From<CertificateAlgorithm> for &str {
  fn from(value: CertificateAlgorithm) -> Self {
    match value {
      CertificateAlgorithm::RSA2048 => RSA_2048_ALGO_NAME,
      CertificateAlgorithm::ECPrime256v1 => EC_PRIME_ALGO_NAME,
    }
  }
}
impl From<CertificateAlgorithm> for String {
  fn from(value: CertificateAlgorithm) -> Self {
    String::from(<&str>::from(value))
  }
}
impl TryFrom<&str> for CertificateAlgorithm {
  type Error = SecurityError;
  fn try_from(value: &str) -> Result<Self, Self::Error> {
    match value {
      RSA_2048_ALGO_NAME => Ok(CertificateAlgorithm::RSA2048),
      EC_PRIME_ALGO_NAME => Ok(CertificateAlgorithm::ECPrime256v1),
      _ => Err(security_error!(
        "Invalid certificate algorithm value: {}",
        value
      )),
    }
  }
}
impl TryFrom<String> for CertificateAlgorithm {
  type Error = SecurityError;
  fn try_from(value: String) -> Result<Self, Self::Error> {
    CertificateAlgorithm::try_from(value.as_str())
  }
}

/// DDS:Auth:PKI-DH IdentityToken type from section 9.3.2.1 of the
/// Security specification (v. 1.1)
///
/// According to the spec, the presence of each of properties is optional
#[derive(Debug, Clone)]
pub(in crate::security) struct BuiltinIdentityToken {
  pub certificate_subject: Option<String>,
  pub certificate_algorithm: Option<CertificateAlgorithm>,
  pub ca_subject: Option<String>,
  pub ca_algorithm: Option<CertificateAlgorithm>,
}

impl TryFrom<IdentityToken> for BuiltinIdentityToken {
  type Error = SecurityError;

  fn try_from(token: IdentityToken) -> Result<Self, Self::Error> {
    let dh = token.data_holder;
    // Verify class id
    if dh.class_id != IDENTITY_TOKEN_CLASS_ID {
      return Err(security_error!(
        "Invalid class ID. Got {}, expected {}",
        dh.class_id,
        IDENTITY_TOKEN_CLASS_ID
      ));
    }

    // Extract properties
    let properties_map = dh.properties_as_map();

    let certificate_subject = if let Some(prop) = properties_map.get(CERT_SN_PROPERTY_NAME) {
      Some(prop.value())
    } else {
      debug!("IdentityToken did not contain the certificate subject name property");
      None
    };

    let certificate_algorithm = if let Some(prop) = properties_map.get(CERT_ALGO_PROPERTY_NAME) {
      Some(CertificateAlgorithm::try_from(prop.value())?)
    } else {
      debug!("IdentityToken did not contain the certificate algorithm property");
      None
    };

    let ca_subject = if let Some(prop) = properties_map.get(CA_SN_PROPERTY_NAME) {
      Some(prop.value())
    } else {
      debug!("IdentityToken did not contain the CA subject name property");
      None
    };

    let ca_algorithm = if let Some(prop) = properties_map.get(CA_ALGO_PROPERTY_NAME) {
      Some(CertificateAlgorithm::try_from(prop.value())?)
    } else {
      debug!("IdentityToken did not contain the CA algorithm property");
      None
    };

    let builtin_token = Self {
      certificate_subject,
      certificate_algorithm,
      ca_subject,
      ca_algorithm,
    };
    Ok(builtin_token)
  }
}

impl From<BuiltinIdentityToken> for IdentityToken {
  fn from(builtin_token: BuiltinIdentityToken) -> Self {
    // First create the DataHolder
    let dh_builder = DataHolderBuilder::with_class_id(IDENTITY_TOKEN_CLASS_ID.to_string())
      .add_property_opt(
        CERT_SN_PROPERTY_NAME,
        builtin_token.certificate_subject,
        true,
      )
      .add_property_opt(
        CERT_ALGO_PROPERTY_NAME,
        builtin_token.certificate_algorithm.map(String::from),
        true,
      )
      .add_property_opt(CA_SN_PROPERTY_NAME, builtin_token.ca_subject, true)
      .add_property_opt(
        CA_ALGO_PROPERTY_NAME,
        builtin_token.ca_algorithm.map(String::from),
        true,
      );

    IdentityToken::from(dh_builder.build())
  }
}

const IDENTITY_STATUS_TOKEN_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0";

const OCSP_STATUS_PROPERTY_NAME: &str = "ocsp_status";

/// DDS:Auth:PKI-DH IdentityStatusToken type from section 9.3.2.2 of the
/// Security specification (v. 1.1)
pub struct BuiltinIdentityStatusToken {
  ocsp_status: Option<String>, // Optional according to spec
}

impl TryFrom<IdentityStatusToken> for BuiltinIdentityStatusToken {
  type Error = String;

  fn try_from(token: IdentityStatusToken) -> Result<Self, Self::Error> {
    let dh = token.data_holder;
    // Verify class id
    if dh.class_id != IDENTITY_STATUS_TOKEN_CLASS_ID {
      return Err(format!(
        "Invalid class ID. Got {}, expected {}",
        dh.class_id, IDENTITY_STATUS_TOKEN_CLASS_ID
      ));
    }

    // Extract OCSP status property if present
    let properties_map = dh.properties_as_map();

    let ocsp_status = if let Some(prop) = properties_map.get(OCSP_STATUS_PROPERTY_NAME) {
      Some(prop.value())
    } else {
      debug!("IdentityStatusToken did not contain OCSP status");
      None
    };

    let builtin_token = Self { ocsp_status };
    Ok(builtin_token)
  }
}

impl From<BuiltinIdentityStatusToken> for IdentityStatusToken {
  fn from(builtin_token: BuiltinIdentityStatusToken) -> Self {
    // First create the DataHolder
    let dh_builder = DataHolderBuilder::with_class_id(IDENTITY_STATUS_TOKEN_CLASS_ID.to_string())
      .add_property_opt(OCSP_STATUS_PROPERTY_NAME, builtin_token.ocsp_status, true);

    IdentityStatusToken::from(dh_builder.build())
  }
}

const AUTH_REQUEST_MESSAGE_TOKEN_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0+AuthReq";

const FUTURE_CHALLENGE_PROPERTY_NAME: &str = "future_challenge";

/// DDS:Auth:PKI-DH AuthRequestMessageToken type from section 9.3.2.4 of the
/// Security specification (v. 1.1)
pub struct BuiltinAuthRequestMessageToken {
  // In spec future_challenge is a property (string value), but it probably needs to be a binary
  // property
  future_challenge: Bytes, // In spec this is
}

impl TryFrom<AuthRequestMessageToken> for BuiltinAuthRequestMessageToken {
  type Error = String;

  fn try_from(token: AuthRequestMessageToken) -> Result<Self, Self::Error> {
    let dh = token.data_holder;
    // Verify class id
    if dh.class_id != AUTH_REQUEST_MESSAGE_TOKEN_CLASS_ID {
      return Err(format!(
        "Invalid class ID. Got {}, expected {}",
        dh.class_id, AUTH_REQUEST_MESSAGE_TOKEN_CLASS_ID
      ));
    }

    // Extract future challenge property
    let bin_properties_map = dh.binary_properties_as_map();

    let future_challenge =
      if let Some(prop) = bin_properties_map.get(FUTURE_CHALLENGE_PROPERTY_NAME) {
        let bytes = prop.value();
        // Check NONCE length is 256 bits / 32 bytes
        if bytes.len() != 32 {
          return Err(format!(
            "Invalid NONCE length. Got {} bytes, expected 32",
            bytes.len()
          ));
        }
        bytes
      } else {
        return Err("AuthRequestMessageToken did not contain future_challenge".to_string());
      };

    let builtin_token = Self { future_challenge };
    Ok(builtin_token)
  }
}

impl From<BuiltinAuthRequestMessageToken> for AuthRequestMessageToken {
  fn from(builtin_token: BuiltinAuthRequestMessageToken) -> Self {
    DataHolderBuilder::with_class_id(AUTH_REQUEST_MESSAGE_TOKEN_CLASS_ID.to_string())
      .add_binary_property(
        FUTURE_CHALLENGE_PROPERTY_NAME,
        builtin_token.future_challenge,
        true,
      )
      .build()
      .into()
  }
}

pub const HANDSHAKE_REQUEST_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0+Req";
pub const HANDSHAKE_REPLY_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0+Reply";
pub const HANDSHAKE_FINAL_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0+Final";

/// DDS:Auth:PKI-DH HandshakeMessageToken type from section 9.3.2.5 of the
/// Security specification (v. 1.1)
/// Works as all three token formats: HandshakeRequestMessageToken,
/// HandshakeReplyMessageToken and HandshakeFinalMessageToken
pub (in crate::security) struct BuiltinHandshakeMessageToken {
  pub class_id: String,
  pub c_id: Option<Bytes>,
  pub c_perm: Option<Bytes>,
  pub c_pdata: Option<Bytes>,
  pub c_dsign_algo: Option<Bytes>,
  pub c_kagree_algo: Option<Bytes>,
  pub ocsp_status: Option<Bytes>,
  pub hash_c1: Option<Bytes>,
  pub dh1: Option<Bytes>,
  pub hash_c2: Option<Bytes>,
  pub dh2: Option<Bytes>,
  pub challenge1: Option<Bytes>,
  pub challenge2: Option<Bytes>,
  pub signature: Option<Bytes>,
}

impl TryFrom<HandshakeMessageToken> for BuiltinHandshakeMessageToken {
  type Error = String;

  fn try_from(token: HandshakeMessageToken) -> Result<Self, Self::Error> {
    let dh = token.data_holder;

    // Verify class id is one of expected
    if !([
      HANDSHAKE_REQUEST_CLASS_ID,
      HANDSHAKE_REPLY_CLASS_ID,
      HANDSHAKE_FINAL_CLASS_ID,
    ]
    .contains(&dh.class_id.as_str()))
    {
      return Err(format!("Invalid class ID '{}'", dh.class_id));
    }

    // Extract binary properties
    let bin_properties_map = dh.binary_properties_as_map();

    let c_id = bin_properties_map.get("c.id").map(|val| val.value());
    let c_perm = bin_properties_map.get("c.perm").map(|val| val.value());
    let c_pdata = bin_properties_map.get("c.pdata").map(|val| val.value());
    let c_dsign_algo = bin_properties_map
      .get("c.dsign_algo")
      .map(|val| val.value());
    let c_kagree_algo = bin_properties_map
      .get("c.kagree_algo")
      .map(|val| val.value());
    let ocsp_status = bin_properties_map.get("ocsp_status").map(|val| val.value());
    let hash_c1 = bin_properties_map.get("hash_c1").map(|val| val.value());
    let dh1 = bin_properties_map.get("dh1").map(|val| val.value());
    let hash_c2 = bin_properties_map.get("hash_c2").map(|val| val.value());
    let dh2 = bin_properties_map.get("dh2").map(|val| val.value());
    let challenge1 = bin_properties_map.get("challenge1").map(|val| val.value());
    let challenge2 = bin_properties_map.get("challenge2").map(|val| val.value());
    let signature = bin_properties_map.get("signature").map(|val| val.value());

    let builtin_token = Self {
      class_id: dh.class_id,
      c_id,
      c_perm,
      c_pdata,
      c_dsign_algo,
      c_kagree_algo,
      ocsp_status,
      hash_c1,
      dh1,
      hash_c2,
      dh2,
      challenge1,
      challenge2,
      signature,
    };
    Ok(builtin_token)
  }
}

impl From<BuiltinHandshakeMessageToken> for HandshakeMessageToken {
  fn from(builtin_token: BuiltinHandshakeMessageToken) -> Self {
    DataHolderBuilder::with_class_id(builtin_token.class_id)
      .add_binary_property_opt("c.id", builtin_token.c_id, true)
      .add_binary_property_opt("c.perm", builtin_token.c_perm, true)
      .add_binary_property_opt("c.pdata", builtin_token.c_pdata, true)
      .add_binary_property_opt("c.dsign_algo", builtin_token.c_dsign_algo, true)
      .add_binary_property_opt("c.kagree_algo", builtin_token.c_kagree_algo, true)
      .add_binary_property_opt("ocsp_status", builtin_token.ocsp_status, true)
      .add_binary_property_opt("hash_c1", builtin_token.hash_c1, true)
      .add_binary_property_opt("dh1", builtin_token.dh1, true)
      .add_binary_property_opt("hash_c2", builtin_token.hash_c2, true)
      .add_binary_property_opt("dh2", builtin_token.dh2, true)
      .add_binary_property_opt("challenge1", builtin_token.challenge1, true)
      .add_binary_property_opt("challenge2", builtin_token.challenge2, true)
      .add_binary_property_opt("signature", builtin_token.signature, true)
      .build()
      .into()
  }
}

const AUTHENTICATED_PEER_TOKEN_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0";

pub(in crate::security) const AUTHENTICATED_PEER_TOKEN_IDENTITY_CERTIFICATE_PROPERTY_NAME: &str =
  "c.id";
pub(in crate::security) const AUTHENTICATED_PEER_TOKEN_PERMISSIONS_DOCUMENT_PROPERTY_NAME: &str =
  "c.perm";

/// DDS:Auth:PKI-DH AuthenticatedPeerCredentialToken type from section 9.3.2.3
/// of the Security specification (v. 1.1)
pub struct BuiltinAuthenticatedPeerCredentialToken {
  c_id: String,
  c_perm: String,
}

impl TryFrom<AuthenticatedPeerCredentialToken> for BuiltinAuthenticatedPeerCredentialToken {
  type Error = String;

  fn try_from(token: AuthenticatedPeerCredentialToken) -> Result<Self, Self::Error> {
    let dh = token.data_holder;
    // Verify class id
    if dh.class_id != AUTHENTICATED_PEER_TOKEN_CLASS_ID {
      return Err(format!(
        "Invalid class ID. Got {}, expected {}",
        dh.class_id, AUTHENTICATED_PEER_TOKEN_CLASS_ID
      ));
    }

    // Extract properties
    let properties_map = dh.properties_as_map();

    let c_id = if let Some(prop) =
      properties_map.get(AUTHENTICATED_PEER_TOKEN_IDENTITY_CERTIFICATE_PROPERTY_NAME)
    {
      prop.value()
    } else {
      return Err(format!(
        "No required {} property",
        AUTHENTICATED_PEER_TOKEN_IDENTITY_CERTIFICATE_PROPERTY_NAME
      ));
    };

    let c_perm = if let Some(prop) =
      properties_map.get(AUTHENTICATED_PEER_TOKEN_PERMISSIONS_DOCUMENT_PROPERTY_NAME)
    {
      prop.value()
    } else {
      return Err(format!(
        "No required {} property",
        AUTHENTICATED_PEER_TOKEN_PERMISSIONS_DOCUMENT_PROPERTY_NAME
      ));
    };

    let builtin_token = Self { c_id, c_perm };
    Ok(builtin_token)
  }
}

impl From<BuiltinAuthenticatedPeerCredentialToken> for AuthenticatedPeerCredentialToken {
  fn from(builtin_token: BuiltinAuthenticatedPeerCredentialToken) -> Self {
    let dh_builder =
      DataHolderBuilder::with_class_id(AUTHENTICATED_PEER_TOKEN_CLASS_ID.to_string())
        .add_property(
          AUTHENTICATED_PEER_TOKEN_IDENTITY_CERTIFICATE_PROPERTY_NAME,
          builtin_token.c_id,
          true,
        )
        .add_property(
          AUTHENTICATED_PEER_TOKEN_PERMISSIONS_DOCUMENT_PROPERTY_NAME,
          builtin_token.c_perm,
          true,
        );
    AuthenticatedPeerCredentialToken::from(dh_builder.build())
  }
}
