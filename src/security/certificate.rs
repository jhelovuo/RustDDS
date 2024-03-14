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

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use bytes::Bytes;
use x509_certificate::{
  certificate::CapturedX509Certificate, EcdsaCurve, KeyAlgorithm, SignatureAlgorithm,
};
use der::Decode;
use bcder::{encode::Values, Mode};

use crate::security::{
  authentication::authentication_builtin::types::{
    CertificateAlgorithm, ECDSA_SIGNATURE_ALGO_NAME, RSA_2048_KEY_LENGTH, RSA_SIGNATURE_ALGO_NAME,
  },
  config::{to_config_error_other, to_config_error_parse, ConfigError},
  types::{security_error, SecurityResult},
};

#[cfg(feature = "security_in_fastdds_compatibility_mode")]
fn rfc4514_to_openssl_oneline(dn_rfc4514: &str) -> String {
  // This function converts a RFC 4514 string representation of a Distinguished
  // Name to the format produced by the legacy OpenSSL function
  // x509_name_oneline, which FastDDS uses. The conversion is needed for
  // interoperability.
  // The fuction
  //  - unescapes characters according to RFC 4514
  //  - reverses the RDNs order
  //  - use '/' as a separator instead of comma
  // The conversion isn't perfect, but works in simple cases which hopeafully is
  // enough.

  let mut result = String::new();
  let mut chars = dn_rfc4514.chars().peekable();

  while let Some(c) = chars.next() {
    match c {
      ',' => {
        // Check if the comma is escaped (preceded by a backslash)
        if result.ends_with('\\') {
          // Remove the escape character and append the comma
          result.pop();
          result.push(',');
        } else {
          // Otherwise, it's an RDN separator, so append a slash
          result.push('/');
        }
      }
      '\\' => {
        // Check for the next character after the escape
        if let Some(&next_char) = chars.peek() {
          match next_char {
            // Include only the escaped character in the result
            ',' | '+' | '\"' | '\\' | '<' | '>' | ';' => {
              chars.next(); // Consume the escaped character
              result.push(next_char);
            }
            // If the escaped character is not one of the above, include the backslash
            _ => result.push('\\'),
          }
        }
      }
      _ => result.push(c), // All other characters are included as-is
    }
  }

  // Reverse the RDNs since OpenSSL one-line format lists RDNs in reverse order
  let rdns: Vec<&str> = result.split('/').rev().collect();
  result = rdns.join("/");

  // Prepend a leading slash if the result is not empty
  if !result.is_empty() {
    result.insert(0, '/');
  }

  // Special handling for the EMAIL attribute
  if result.contains("EMAIL=") {
    result = result.replace("EMAIL=", "emailAddress=");
  }

  result
}

// This is mostly a wrapper around
// x509_certificate::certificate::CapturedX509Certificate
// so that we can keep track of what operations we use.
#[derive(Clone, Debug)]
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

  pub fn to_pem(&self) -> String {
    self.cert.encode_pem()
  }

  // public key algorithm
  pub fn key_algorithm(&self) -> Option<x509_certificate::KeyAlgorithm> {
    self.cert.key_algorithm()
  }

  // name of the signature algoritm as a byte string accrding to Table 49
  // in DDS Security Spec v1.1 Section "9.3.2.5.1 HandshakeRequestMessageToken
  // objects"
  pub fn signature_algorithm_identifier(&self) -> SecurityResult<Bytes> {
    match self.cert.signature_algorithm() {
      None => Err(security_error(
        "Certificate has no known signature algorithm?!",
      )),
      Some(SignatureAlgorithm::RsaSha256) => Ok(Bytes::from_static(RSA_SIGNATURE_ALGO_NAME)),
      Some(SignatureAlgorithm::EcdsaSha256) => Ok(Bytes::from_static(ECDSA_SIGNATURE_ALGO_NAME)),
      Some(x) => Err(security_error(&format!(
        "Certificate has out-of-spec signature algorithm {:?}",
        x
      ))),
    }
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
  // e.g.
  // `someones_identity.verify_signed_by_certificate( certificate_authority )`
  //
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
pub struct DistinguishedName(x509_cert::name::DistinguishedName);
impl DistinguishedName {
  pub fn parse(s: &str) -> Result<DistinguishedName, ConfigError> {
    x509_cert::name::DistinguishedName::from_str(s)
      .map(DistinguishedName)
      .map_err(|e| ConfigError::Parse(format!("Error parsing DistinguishedName: {e:?}")))
  }

  pub fn serialize(&self) -> String {
    let rfc4514_string = self.0.to_string();
    #[cfg(not(feature = "security_in_fastdds_compatibility_mode"))]
    {
      rfc4514_string
    }
    #[cfg(feature = "security_in_fastdds_compatibility_mode")]
    {
      rfc4514_to_openssl_oneline(&rfc4514_string)
    }
  }

  // TODO is this a too strict equivalence?
  pub fn matches(&self, other: &Self) -> bool {
    self.0 == other.0
  }
}

// This conversion should be non-fallible?
impl From<x509_cert::name::Name> for DistinguishedName {
  fn from(name: x509_cert::name::Name) -> DistinguishedName {
    DistinguishedName(name)
  }
}

use std::{fmt, str::FromStr};

impl fmt::Display for DistinguishedName {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
    write!(f, "{}", self.serialize())
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
}
