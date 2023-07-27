use bytes::Bytes;
use log::debug;

use crate::security::DataHolderBuilder;
use super::{
  types::{IdentityStatusToken, IdentityToken},
  AuthRequestMessageToken, AuthenticatedPeerCredentialToken, HandshakeMessageToken,
};

const IDENTITY_TOKEN_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0";

// Expected property names in IdentityToken
const CERT_SN_PROPERTY_NAME: &str = "dds.cert.sn";
const CERT_ALGO_PROPERTY_NAME: &str = "dds.cert.algo";
const CA_SN_PROPERTY_NAME: &str = "dds.ca.sn";
const CA_ALGO_PROPERTY_NAME: &str = "dds.ca.algo";

// Accepted values for the algorithm properties in IdentityToken
const RSA_2048_ALGO_NAME: &str = "RSA-2048";
const EC_PRIME_ALGO_NAME: &str = "EC-prime256v1";

enum CertificateAlgorithm {
  RSA2048,
  ECPrime256v1,
}

/// DDS:Auth:PKI-DH IdentityToken type from section 9.3.2.1 of the
/// Security specification (v. 1.1)
///
/// According to the spec, the presence of each of properties is optional
pub struct BuiltinIdentityToken {
  certificate_subject: Option<String>,
  certificate_algorithm: Option<CertificateAlgorithm>,
  ca_subject: Option<String>,
  ca_algorithm: Option<CertificateAlgorithm>,
}

impl TryFrom<IdentityToken> for BuiltinIdentityToken {
  type Error = String;

  fn try_from(token: IdentityToken) -> Result<Self, Self::Error> {
    let dh = token.data_holder;
    // Verify class id
    if dh.class_id != IDENTITY_TOKEN_CLASS_ID {
      return Err(format!(
        "Invalid class ID. Got {}, expected {}",
        dh.class_id, IDENTITY_TOKEN_CLASS_ID
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
      match prop.value().as_str() {
        RSA_2048_ALGO_NAME => Some(CertificateAlgorithm::RSA2048),
        EC_PRIME_ALGO_NAME => Some(CertificateAlgorithm::ECPrime256v1),
        _ => {
          return Err(format!(
            "Invalid certificate algorithm value: {}",
            prop.value()
          ));
        }
      }
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
      match prop.value().as_str() {
        RSA_2048_ALGO_NAME => Some(CertificateAlgorithm::RSA2048),
        EC_PRIME_ALGO_NAME => Some(CertificateAlgorithm::ECPrime256v1),
        _ => {
          return Err(format!("Invalid CA algorithm value: {}", prop.value()));
        }
      }
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
    let mut dh_builder = DataHolderBuilder::new();

    // Set class id
    dh_builder.set_class_id(IDENTITY_TOKEN_CLASS_ID);

    // Add certificate subject name if present
    if let Some(val) = builtin_token.certificate_subject {
      dh_builder.add_property(CERT_SN_PROPERTY_NAME, val, true);
    }

    // Add certificate algorithm if present
    if let Some(val) = builtin_token.certificate_algorithm {
      let algo_value = match val {
        CertificateAlgorithm::RSA2048 => RSA_2048_ALGO_NAME.to_string(),
        CertificateAlgorithm::ECPrime256v1 => EC_PRIME_ALGO_NAME.to_string(),
      };
      dh_builder.add_property(CERT_ALGO_PROPERTY_NAME, algo_value, true);
    }

    // Add CA subject name if present
    if let Some(val) = builtin_token.ca_subject {
      dh_builder.add_property(CA_SN_PROPERTY_NAME, val, true);
    }

    // Add CA algorithm if present
    if let Some(val) = builtin_token.ca_algorithm {
      let algo_value = match val {
        CertificateAlgorithm::RSA2048 => RSA_2048_ALGO_NAME.to_string(),
        CertificateAlgorithm::ECPrime256v1 => EC_PRIME_ALGO_NAME.to_string(),
      };
      dh_builder.add_property(CA_ALGO_PROPERTY_NAME, algo_value, true);
    }

    // Build the DataHolder and create the IdentityToken from it
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
    let mut dh_builder = DataHolderBuilder::new();

    // Set class id
    dh_builder.set_class_id(IDENTITY_STATUS_TOKEN_CLASS_ID);

    // Add OCSP status if present
    if let Some(val) = builtin_token.ocsp_status {
      dh_builder.add_property(OCSP_STATUS_PROPERTY_NAME, val, true);
    }

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
    // First create the DataHolder
    let mut dh_builder = DataHolderBuilder::new();

    // Set class id
    dh_builder.set_class_id(AUTH_REQUEST_MESSAGE_TOKEN_CLASS_ID);

    // Add future status property
    dh_builder.add_binary_property(
      FUTURE_CHALLENGE_PROPERTY_NAME,
      builtin_token.future_challenge,
      true,
    );

    AuthRequestMessageToken::from(dh_builder.build())
  }
}

const HANDSHAKE_REQUEST_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0+Req";
const HANDSHAKE_REPLY_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0+Reply";
const HANDSHAKE_FINAL_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0+Final";

/// DDS:Auth:PKI-DH HandshakeMessageToken type from section 9.3.2.5 of the
/// Security specification (v. 1.1)
/// Works as all three token formats: HandshakeRequestMessageToken,
/// HandshakeReplyMessageToken and HandshakeFinalMessageToken
pub struct BuiltinHandshakeMessageToken {
  class_id: String,
  c_id: Option<Bytes>,
  c_perm: Option<Bytes>,
  c_pdata: Option<Bytes>,
  c_dsign_algo: Option<Bytes>,
  c_kagree_algo: Option<Bytes>,
  ocsp_status: Option<Bytes>,
  hash_c1: Option<Bytes>,
  dh1: Option<Bytes>,
  hash_c2: Option<Bytes>,
  dh2: Option<Bytes>,
  challenge1: Option<Bytes>,
  challenge2: Option<Bytes>,
  signature: Option<Bytes>,
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
    // First create the DataHolder
    let mut dh_builder = DataHolderBuilder::new();

    // Set class id
    dh_builder.set_class_id(&builtin_token.class_id);

    // Add the various binary properties if present

    if let Some(val) = builtin_token.c_id {
      dh_builder.add_binary_property("c.id", val, true);
    }
    if let Some(val) = builtin_token.c_perm {
      dh_builder.add_binary_property("c.perm", val, true);
    }
    if let Some(val) = builtin_token.c_pdata {
      dh_builder.add_binary_property("c.pdata", val, true);
    }
    if let Some(val) = builtin_token.c_dsign_algo {
      dh_builder.add_binary_property("c.dsign_algo", val, true);
    }
    if let Some(val) = builtin_token.c_kagree_algo {
      dh_builder.add_binary_property("c.kagree_algo", val, true);
    }
    if let Some(val) = builtin_token.ocsp_status {
      dh_builder.add_binary_property("ocsp_status", val, true);
    }
    if let Some(val) = builtin_token.hash_c1 {
      dh_builder.add_binary_property("hash_c1", val, true);
    }
    if let Some(val) = builtin_token.dh1 {
      dh_builder.add_binary_property("dh1", val, true);
    }
    if let Some(val) = builtin_token.hash_c2 {
      dh_builder.add_binary_property("hash_c2", val, true);
    }
    if let Some(val) = builtin_token.dh2 {
      dh_builder.add_binary_property("dh2", val, true);
    }
    if let Some(val) = builtin_token.challenge1 {
      dh_builder.add_binary_property("challenge1", val, true);
    }
    if let Some(val) = builtin_token.challenge2 {
      dh_builder.add_binary_property("challenge2", val, true);
    }
    if let Some(val) = builtin_token.signature {
      dh_builder.add_binary_property("signature", val, true);
    }

    HandshakeMessageToken::from(dh_builder.build())
  }
}

const AUTHENTICATED_PEER_TOKEN_CLASS_ID: &str = "DDS:Auth:PKI-DH:1.0";

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

    let c_id = if let Some(prop) = properties_map.get("c.id") {
      prop.value()
    } else {
      return Err("No required c.id property".to_string());
    };

    let c_perm = if let Some(prop) = properties_map.get("c.perm") {
      prop.value()
    } else {
      return Err("No required c.perm property".to_string());
    };

    let builtin_token = Self { c_id, c_perm };
    Ok(builtin_token)
  }
}

impl From<BuiltinAuthenticatedPeerCredentialToken> for AuthenticatedPeerCredentialToken {
  fn from(builtin_token: BuiltinAuthenticatedPeerCredentialToken) -> Self {
    // First create the DataHolder
    let mut dh_builder = DataHolderBuilder::new();

    // Set class id
    dh_builder.set_class_id(AUTHENTICATED_PEER_TOKEN_CLASS_ID);

    // Add properties
    dh_builder.add_property("c.id", builtin_token.c_id, true);
    dh_builder.add_property("c.perm", builtin_token.c_perm, true);

    AuthenticatedPeerCredentialToken::from(dh_builder.build())
  }
}
