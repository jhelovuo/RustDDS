use std::{
  borrow::Borrow,
  path::{Path, PathBuf},
};

use crate::{qos, security};

pub enum PrivateSigningKey {
  /// Private key is stored in a regular .pem file. The contents may be
  /// encrypted with a pasword.
  Files {
    /// Where the PEM file is stored
    file_path: PathBuf,
    /// Decryption key, if private key file is encrypted
    file_password: String,
  },
  /// The private key is held by a Hardware Security Module, which typically
  /// refuses to output the key.
  /// Signing operations are done by the HSM.
  Pkcs11 {
    /// Dynamic library file for accessing the HSM, e.g.
    /// "/usr/lib/softhsm/libsofthsm2.so" Use absolute path.
    hsm_access_library: PathBuf,

    /// Label of the token to use. See PKCS#11 spec Section "3.2 Slot and token
    /// types" definition of CK_TOKEN_INFO for the meaning of "label".
    token_label: String,

    /// Login PIN code to operate the token, if any.
    /// Despite the name, the PIN is alphanumeric.
    /// It is used to attempt login as "user" (not Security Officer).
    token_pin: Option<String>,
  },
}

/// This holds the paths to files that configure DDS Security.
pub struct DomainParticipantSecurityConfigFiles {
  /// CA that is used to validate identities of DomainParticipants
  pub identity_ca_certificate: PathBuf,
  /// Identity docuemnt for this Participant
  pub participant_identity_certificate: PathBuf,
  /// Private (signing) key for this participant
  pub participant_identity_private_key: PrivateSigningKey,
  /// CA that is used to validate permissions documents. May be the same as
  /// Identity CA above.
  pub permissions_ca_certificate: PathBuf,
  /// Access permissions/rules for the DDS Domains to be joined.
  pub domain_governance_document: PathBuf,
  /// Access control rules for topics and participants.
  pub participant_permissions_document: PathBuf,
  /// CRLs are not yet implemented.
  pub certificate_revocation_list: Option<PathBuf>,
}

impl DomainParticipantSecurityConfigFiles {
  /// Assign some default names to security config files.
  pub fn with_ros_default_names(
    security_config_dir: impl AsRef<Path>,
    private_key_password: String,
  ) -> Self {
    let d = security_config_dir;

    // The default names are taken from
    // https://github.com/ros2/rmw_dds_common/blob/6fae970a99c3d4e0684a6e987edb89505b8ee213/rmw_dds_common/src/security.cpp#L25
    DomainParticipantSecurityConfigFiles {
      identity_ca_certificate: own_and_append(&d, "identity_ca.cert.pem"),
      participant_identity_certificate: own_and_append(&d, "cert.pem"),
      participant_identity_private_key: PrivateSigningKey::Files {
        file_path: own_and_append(&d, "key.pem"),
        file_password: private_key_password,
      },
      permissions_ca_certificate: own_and_append(&d, "permissions_ca.cert.pem"),
      domain_governance_document: own_and_append(&d, "governance.p7s"),
      participant_permissions_document: own_and_append(&d, "permissions.p7s"),
      certificate_revocation_list: None, // "crl.pem"
    }
  }

  pub fn with_ros_default_names_and_hsm(
    security_config_dir: impl AsRef<Path>,
    hsm_access_library: impl AsRef<Path>,
    token_label: String,
    token_pin: Option<String>,
  ) -> Self {
    let d = security_config_dir;

    // The default names are taken from
    // https://github.com/ros2/rmw_dds_common/blob/6fae970a99c3d4e0684a6e987edb89505b8ee213/rmw_dds_common/src/security.cpp#L25
    DomainParticipantSecurityConfigFiles {
      identity_ca_certificate: own_and_append(&d, "identity_ca.cert.pem"),
      participant_identity_certificate: own_and_append(&d, "cert.pem"),
      participant_identity_private_key: PrivateSigningKey::Pkcs11 {
        hsm_access_library: own_and_append("", hsm_access_library),
        token_label,
        token_pin,
      },
      permissions_ca_certificate: own_and_append(&d, "permissions_ca.cert.pem"),
      domain_governance_document: own_and_append(&d, "governance.p7s"),
      participant_permissions_document: own_and_append(&d, "permissions.p7s"),
      certificate_revocation_list: None, // "crl.pem"
    }
  }

  pub fn into_property_policy(self) -> qos::policy::Property {
    let mut value = vec![
      mk_file_prop("dds.sec.auth.identity_ca", &self.identity_ca_certificate),
      mk_file_prop(
        "dds.sec.auth.identity_certificate",
        &self.participant_identity_certificate,
      ),
      match self.participant_identity_private_key {
        PrivateSigningKey::Files { ref file_path, .. } => {
          mk_file_prop("dds.sec.auth.private_key", file_path)
        }
        PrivateSigningKey::Pkcs11 {
          ref token_label,
          ref token_pin,
          ..
        } => {
          // for example
          // pkcs11:object=my_private_key_name?pin-value=OpenSesame
          let mut pkcs11_uri = format!("pkcs11:object={}", token_label);
          if let Some(pin) = token_pin {
            pkcs11_uri.push_str(&format!("?pin-value={}", pin));
          }
          mk_string_prop("dds.sec.auth.private_key", pkcs11_uri)
        }
      },
      mk_file_prop(
        "dds.sec.access.permissions_ca",
        &self.permissions_ca_certificate,
      ),
      mk_file_prop(
        "dds.sec.access.governance",
        &self.domain_governance_document,
      ),
      mk_file_prop(
        "dds.sec.access.permissions",
        &self.participant_permissions_document,
      ),
    ];
    if let PrivateSigningKey::Files { file_password, .. } = self.participant_identity_private_key {
      value.push(mk_string_prop("dds.sec.auth.password", file_password));
    }

    qos::policy::Property {
      value,
      binary_value: vec![],
    }
  }
}

fn own_and_append(d: impl AsRef<Path>, f: impl AsRef<Path>) -> PathBuf {
  let mut pb = d.as_ref().to_path_buf();
  pb.push(f);
  pb
}

fn mk_file_prop(name: &str, file_path: impl AsRef<Path>) -> security::types::Property {
  let mut value = "file:".to_string();
  value.push_str(file_path.as_ref().to_string_lossy().borrow());

  security::types::Property {
    name: name.to_string(),
    value,
    propagate: false,
  }
}

fn mk_string_prop(name: &str, value: String) -> security::types::Property {
  security::types::Property {
    name: name.to_string(),
    value,
    propagate: false,
  }
}

use bytes::Bytes;

pub(crate) fn read_uri(uri: &str) -> Result<Bytes, ConfigError> {
  match uri.split_once(':') {
    Some(("data", content)) => Ok(Bytes::copy_from_slice(content.as_bytes())),
    Some(("pkcs11", _)) => Err(other_config_error(
      "Config URI schema 'pkcs11:' not implemented.".to_owned(),
    )),
    Some(("file", path)) => std::fs::read(path)
      .map_err(to_config_error_other(&format!("I/O error reading {path}")))
      .map(Bytes::from),
    _ => Err(parse_config_error(
      "Config URI must begin with 'file:' , 'data:', or 'pkcs11:'.".to_owned(),
    )),
  }
}

use std::fmt::Debug;

#[derive(Debug)]
pub enum ConfigError {
  Parse(String),
  Pkcs7(String),
  Security(String),
  Other(String),
}

impl From<glob::PatternError> for ConfigError {
  fn from(e: glob::PatternError) -> ConfigError {
    ConfigError::Parse(format!("Bad glob pattern: {e:?}"))
  }
}

impl From<serde_xml_rs::Error> for ConfigError {
  fn from(e: serde_xml_rs::Error) -> ConfigError {
    ConfigError::Parse(format!("XML parse error: {e:?}"))
  }
}

pub(crate) fn to_config_error_other<E: Debug + 'static>(
  text: &str,
) -> impl FnOnce(E) -> ConfigError + '_ {
  move |e: E| ConfigError::Other(format!("{}: {:?}", text, e))
}

pub(crate) fn to_config_error_pkcs7<E: Debug + 'static>(
  text: &str,
) -> impl FnOnce(E) -> ConfigError + '_ {
  move |e: E| ConfigError::Pkcs7(format!("{}: {:?}", text, e))
}

pub(crate) fn to_config_error_parse<E: Debug + 'static>(
  text: &str,
) -> impl FnOnce(E) -> ConfigError + '_ {
  move |e: E| ConfigError::Parse(format!("{}: {:?}", text, e))
}

pub(crate) fn parse_config_error(text: String) -> ConfigError {
  ConfigError::Parse(text)
}

pub(crate) fn other_config_error(text: String) -> ConfigError {
  ConfigError::Other(text)
}

pub(crate) fn pkcs7_config_error(text: String) -> ConfigError {
  ConfigError::Pkcs7(text)
}
