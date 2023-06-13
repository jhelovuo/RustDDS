use log::debug;

use crate::security::DataHolderBuilder;
use super::types::IdentityToken;

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
