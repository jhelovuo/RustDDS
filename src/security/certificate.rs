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
use std::collections::BTreeMap;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use bytes::Bytes;
use x509_certificate::{
  certificate::CapturedX509Certificate, signing::InMemorySigningKeyPair, EcdsaCurve, KeyAlgorithm,
  Signer,
};
use der::Decode;
use bcder::{encode::Values, Mode};

use crate::security::{
  authentication::authentication_builtin::types::{CertificateAlgorithm, RSA_2048_KEY_LENGTH},
  config::{parse_config_error, to_config_error_other, to_config_error_parse, ConfigError},
  types::{security_error, SecurityResult},
};

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
    self.0.to_string()
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

use cryptoki::{
  context::{CInitializeArgs, Pkcs11},
  mechanism::Mechanism,
  object::{Attribute, AttributeType, ObjectClass, ObjectHandle},
  session::{Session, UserType},
  slot::Slot,
  types::AuthPin,
};

#[derive(Debug)]
pub(crate) enum PrivateKey {
  InMemory {
    priv_key: InMemorySigningKeyPair,
  },
  InHSM {
    // We store context and slot just in case dropping them would close them.
    #[allow(dead_code)]
    context: Pkcs11,
    #[allow(dead_code)]
    slot: Slot,
    session: Session,
    key_object_handle: ObjectHandle,
  },
}

// TODO: decrypt a password protected key
impl PrivateKey {
  pub fn from_pem(pem_data: impl AsRef<[u8]>) -> Result<Self, ConfigError> {
    let priv_key = InMemorySigningKeyPair::from_pkcs8_pem(pem_data.as_ref())
      .map_err(to_config_error_parse("Private key parse error"))?;

    Ok(PrivateKey::InMemory { priv_key })
  }

  // decode object label in path
  // load HSM library
  // initialize library
  // get slots and enumerate
  // find slot where token is present and matches object label
  // open RO session to token and login with PIN
  // find private key objects
  // check that private key object supports "sign"
  // store library handle, slot handle, session handle to PrivateKey object
  // store object handle to PrivateKey
  // return PrivateKey

  // Note: We do not check the key type (RSA, DSA, EC, etc.) against anything.
  // We just trust that the specified key is of the correct type.

  pub fn from_pkcs11_uri_path_and_query(path_and_query: &str) -> Result<Self, ConfigError> {
    let interesting_attributes = vec![
      AttributeType::AllowedMechanisms,
      AttributeType::Class,
      AttributeType::Label,
      AttributeType::KeyType,
      AttributeType::Sign,
      AttributeType::Sensitive,
    ];
    // decode URI: path and possible query (PIN)
    //
    // example path_and_query, which is part of URI after scheme "pkcs11:"
    //
    // token=my_token_label?pin-value=1234&module-path=/usr/lib/softhsm/libsofthsm2.
    // so
    //
    // The "module-path" is a query-attribute like the PIN, and not a path-attribute
    // like "token", which may seem a bit strange choice, but that is what RFC
    // 7512 says.
    let (path, query) = path_and_query
      .split_once('?')
      .unwrap_or((path_and_query, ""));
    let path_attrs: BTreeMap<&str, &str> =
      path.split(';').filter_map(|a| a.split_once('=')).collect();
    let query_attrs: BTreeMap<&str, &str> =
      query.split('&').filter_map(|a| a.split_once('=')).collect();

    let token_label = path_attrs.get("object").ok_or(parse_config_error(
      "pkcs11 URI must specify attribute \"object\" i.e. token label".to_string(),
    ))?;

    let pin_value_opt = query_attrs.get("pin-value");

    let module_path: &str = query_attrs
      .get("module-path")
      .cloned()
      .unwrap_or("/usr/lib/softhsm/libsofthsm2.so"); // Some semi-reasnoable default value.

    info!("Opening PKCS#11 HSM client library {}", module_path);
    let context = Pkcs11::new(module_path)?;
    context.initialize(CInitializeArgs::OsThreads)?;
    debug!("PKCS#11 HSM library info: {:?}", context.get_library_info());

    //let expected_key_type = KeyType::RSA;

    let slots = context.get_all_slots()?;
    debug!("PKCS#11 HSM found {} slots.", slots.len());
    for (num, slot) in slots.iter().enumerate() {
      let slot_info = context.get_slot_info(*slot);
      match slot_info {
        Ok(si) if si.token_present() => {
          match context.get_token_info(*slot) {
            Ok(token_info) => {
              if token_info.label() == *token_label {
                info!("Found matching token \"{}\" in slot {}.", token_label, num);
                let session = context.open_ro_session(*slot)?;
                let secret_pin_opt: Option<AuthPin> =
                  pin_value_opt.map(|p| AuthPin::from_str(p).unwrap());
                // unwrap is safe, because error type is Infallible
                session.login(UserType::User, secret_pin_opt.as_ref())?; // bail on failure
                info!(
                  "Logged into token \"{}\" , using PIN = {:?}",
                  token_label,
                  secret_pin_opt.is_some()
                );
                for (obj_num, obj) in session.find_objects(&[])?.iter().enumerate() {
                  let attr = session.get_attributes(*obj, &interesting_attributes)?;
                  // Check the attributes. Does this look like a private key?
                  if attr.iter().any(|a| a == &Attribute::Class(ObjectClass::PRIVATE_KEY)) &&
                     //attr.iter().any(|a| a == &Attribute::KeyType(expected_key_type)) &&
                     attr.iter().any(|a| a == &Attribute::Sign(true))
                  {
                    // Looks like a private key. Exit here.
                    let object_label = 
                      attr.iter().find_map(|a| match a {
                          Attribute::Label(bytes) => Some(String::from_utf8_lossy(bytes)),
                          _ => None,
                        }
                      );
                    let allowed_mechanisms = attr.iter().find_map(|a| match a {
                          Attribute::AllowedMechanisms(am_vec) => Some(am_vec),
                          _ => None,
                        }
                      );
                    let key_type = attr.iter().find_map(|a| match a {
                          Attribute::KeyType(kt) => Some(kt),
                          _ => None,
                        }
                      );

                    info!("Object {}: Using as Private Key. label={:?}", obj_num, object_label);
                    info!("Object {}: AllowedMechanisms {:?}", obj_num, allowed_mechanisms);
                    info!("Object {}: KeyType {:?}", obj_num, key_type);
                    return Ok(PrivateKey::InHSM {
                      context,
                      slot: *slot,
                      session,
                      key_object_handle: *obj,
                    });
                  } else {
                    debug!("Object {}: Attributes  do not match {:?}", obj_num, attr);
                  }
                } // for objects
              } else {
                debug!(
                  "Slot {} token label={} . This is not the token we are looking for.",
                  num,
                  token_info.label()
                );
              }
            }
            Err(e) => warn!("Slot {} get_token_info error: {:?}", num, e),
          }
        }
        Ok(_) => info!("Slot {} has no token", num),
        Err(e) => warn!("Slot {} get_slot_info error: {:?}", num, e),
      }
    } // for slots

    Err(parse_config_error(
      "Did not find private key in HSM".to_string(),
    ))
  }

  pub fn sign(&self, msg: &[u8]) -> SecurityResult<Bytes> {
    match self {
      PrivateKey::InMemory { priv_key } => priv_key
        .try_sign(msg)
        .map(|s| Bytes::copy_from_slice(s.as_ref()))
        .map_err(|e| security_error(&format!("Signature verification failure: {e:?}"))),
      PrivateKey::InHSM {
        session,
        key_object_handle,
        ..
      } => {
        let sign_mechanism = Mechanism::EcdsaSha256; // TODO: Other mechanisms also
        Ok(
          session
            .sign(&sign_mechanism, *key_object_handle, msg)
            .map(Bytes::from)?,
        )
      }
    }
  } // fn
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
