//
// DDS Security spec v1.1
//
// Section "9.4.1.1 Permissions CA Certificate
//
// This is an X.509 certificate that contains the Public Key of the CA that will
// be used to sign the Domain Governance and Domain Permissions document. The
// certificate can be self-signed or signed by some other CA. Regardless of
// this the Public Key in the Certificate shall be trusted to sign the
// aforementioned Governance and Permissions documents (see 9.4.1.2 and
// 9.4.1.3). The Permissions CA Certificate shall be provided to the plugins
// using the PropertyQosPolicy on the DomainParticipantQos as specified in
// Table 56."
//

// So this type is to be used to verify both Domain Governance and Domain
// Permissions documents. The verification of the two can use the same or
// different Certificate instances.

use x509_certificate::{
  certificate::CapturedX509Certificate, signing::InMemorySigningKeyPair, EcdsaCurve, KeyAlgorithm,
};
use der::Decode;
use bcder::{encode::Values, Mode};

use crate::security::{
  authentication::authentication_builtin::types::{CertificateAlgorithm, RSA_2048_KEY_LENGTH},
  config::{to_config_error_other, to_config_error_parse, ConfigError},
  types::{security_error, SecurityResult},
};

// This is mostly a wrapper around
// x509_certificate::certificate::CapturedX509Certificate
// so that we can keep track of what operations we use.
#[derive(Debug)]
pub struct Certificate {
  cert: CapturedX509Certificate,
  subject_name: DistinguishedName,
}

impl Certificate {
  pub fn from_pem(pem_data: impl AsRef<[u8]>) -> Result<Self, ConfigError> {
    let cert = CapturedX509Certificate::from_pem(pem_data)
      .map_err(to_config_error_parse("Cannot read X.509 Certificate"))?;

    let other_cert = x509_cert::certificate::Certificate::from_der(cert.constructed_data())
      .map_err(to_config_error_parse("Cannot read X.509 Certificate(2)"))?;

    let subject_name = other_cert.tbs_certificate.subject.into();

    Ok(Certificate { cert, subject_name })
  }

  pub fn subject_name(&self) -> &DistinguishedName {
    &self.subject_name
  }

  pub fn subject_name_der(&self) -> Result<Vec<u8>, ConfigError> {
    let er = &self.cert.subject_name().encode_ref();
    let mut buf = Vec::with_capacity(er.encoded_len(Mode::Der));
    er.write_encoded(Mode::Der, &mut buf)
      .map_err(to_config_error_other(
        "Cannot extract subject_name DER encoding",
      ))?;
    Ok(buf)
  }

  pub(super) fn algorithm(&self) -> Option<CertificateAlgorithm> {
    let key_algorithm = &self.cert.key_algorithm();
    match key_algorithm {
      Some(KeyAlgorithm::Rsa) => {
        let key_length = self.cert.public_key_data().len();
        if key_length == RSA_2048_KEY_LENGTH {
          Some(CertificateAlgorithm::RSA2048)
        } else {
          log::error!(
            "Wrong RSA key length: expected {}, got {}",
            RSA_2048_KEY_LENGTH,
            key_length
          );
          None
        }
      }
      Some(KeyAlgorithm::Ecdsa(EcdsaCurve::Secp256r1)) => Some(CertificateAlgorithm::ECPrime256v1),
      _ => {
        log::error!(
          "Unknown key algorithm. cert.key_algorithm returned {:?}",
          key_algorithm
        );
        None
      }
    }
  }

  pub fn verify_signed_data_with_algorithm(
    &self,
    signed_data: impl AsRef<[u8]>,
    signature: impl AsRef<[u8]>,
    verify_algorithm: &'static dyn ring::signature::VerificationAlgorithm,
  ) -> SecurityResult<()> {
    self
      .cert
      .verify_signed_data_with_algorithm(signed_data, signature, verify_algorithm)
      .map_err(|e| security_error(&format!("Signature verification failure: {e:?}")))
  }

  // Verify that `self` was signed by `other` Certificate
  pub fn verify_signed_by_certificate(&self, other: &Certificate) -> SecurityResult<()> {
    self
      .cert
      .verify_signed_by_certificate(&other.cert)
      .map_err(|e| {
        security_error(&format!(
          "Certificate signature verification failure: {e:?}"
        ))
      })
  }
}

// This represents X.501 Distinguished Name
//
// See https://datatracker.ietf.org/doc/html/rfc4514
//
// It is supposed to be a structured type of key-value-mappings,
// but for simplicity, we treat it just as a string for time being.
// It is needs to process "Subject Name" and "Issuer Name" in
// X.509 Certificates.
//
// Structured representation would allow standards-compliant
// equality comparison (`.matches()`) according to
// https://datatracker.ietf.org/doc/html/rfc5280#section-7.1
//
// TODO: Implement the structured format and matching.
#[derive(Debug, Clone)]
pub struct DistinguishedName {
  name: String,
}

impl DistinguishedName {
  pub fn parse(s: &str) -> Result<DistinguishedName, ConfigError> {
    Ok(DistinguishedName {
      name: s.to_string(),
    })
  }

  // TODO we need serialized subject names in tokens
  pub fn serialize(self) -> String {
    self.name
  }

  pub fn matches(&self, other: &Self) -> bool {
    self.name == other.name
  }
}

// This conversion should be non-fallible?
impl From<x509_cert::name::Name> for DistinguishedName {
  fn from(n: x509_cert::name::Name) -> DistinguishedName {
    DistinguishedName {
      name: format!("{}", n),
    }
  }
}

use std::fmt;

impl fmt::Display for DistinguishedName {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
    write!(f, "{}", self.name)
  }
}

#[derive(Debug)]
pub struct PrivateKey {
  priv_key: InMemorySigningKeyPair,
}

// TODO: decrypt a password protected key
impl PrivateKey {
  pub fn from_pem(pem_data: impl AsRef<[u8]>) -> Result<Self, ConfigError> {
    let priv_key = InMemorySigningKeyPair::from_pkcs8_pem(pem_data.as_ref())
      .map_err(to_config_error_parse("Private key parse error"))?;

    Ok(PrivateKey { priv_key })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn parse_example() {
    let cert_pem = r#"-----BEGIN CERTIFICATE-----
MIIBOzCB4qADAgECAhR361786/qVPfJWWDw4Wg5cmJUwBTAKBggqhkjOPQQDAjAS
MRAwDgYDVQQDDAdzcm9zMkNBMB4XDTIzMDcyMzA4MjgzNloXDTMzMDcyMTA4Mjgz
NlowEjEQMA4GA1UEAwwHc3JvczJDQTBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IA
BMpvJQ/91ZqnmRRteTL2qaEFz2d7SGAQQk9PIhhZCV1tlLwYf/hI4xWLJaEv8FxJ
TjxXRGJ1U+/IqqqIvJVpWaSjFjAUMBIGA1UdEwEB/wQIMAYBAf8CAQEwCgYIKoZI
zj0EAwIDSAAwRQIgEiyVGRc664+/TE/HImA4WNwsSi/alHqPYB58BWINj34CIQDD
iHhbVPRB9Uxts9CwglxYgZoUdGUAxreYIIaLO4yLqw==
-----END CERTIFICATE-----
"#;

    let cert = Certificate::from_pem(cert_pem).unwrap();

    println!("{:?}", cert);
  }

  #[test]
  pub fn parse_private_key() {
    let priv_key_pem = r#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgaPBddesE0rHiP5/c
+djUjctfNoMAa5tNxOdged9AQtOhRANCAATKbyUP/dWap5kUbXky9qmhBc9ne0hg
EEJPTyIYWQldbZS8GH/4SOMViyWhL/BcSU48V0RidVPvyKqqiLyVaVmk
-----END PRIVATE KEY-----
"#;

    let key = PrivateKey::from_pem(priv_key_pem).unwrap();

    println!("{:?}", key);
  }
}
