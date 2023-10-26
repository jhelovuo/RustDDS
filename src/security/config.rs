pub mod paths;
use std::{
  borrow::Borrow,
  path::{Path, PathBuf},
};

use crate::{qos, security};

/// This holds the paths to files that configure DDS Security.
pub struct DomainParticipantSecurityConfigFiles {
  /// CA that is used to validate identities of DomainParticipants
  pub identity_ca_certificate: PathBuf,
  /// Identity docuemnt for this Participant
  pub participant_identity_certificate: PathBuf,
  /// Private (signing) key for this participant
  pub participant_identity_private_key: PathBuf,
  /// Private key password for this participant
  pub private_key_password: String,
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
      participant_identity_private_key: own_and_append(&d, "key.pem"),
      private_key_password,
      permissions_ca_certificate: own_and_append(&d, "permissions_ca.cert.pem"),
      domain_governance_document: own_and_append(&d, "governance.p7s"),
      participant_permissions_document: own_and_append(&d, "permissions.p7s"),
      certificate_revocation_list: None, // "crl.pem"
    }
  }

  pub fn into_property_policy(self) -> qos::policy::Property {
    qos::policy::Property {
      value: vec![
        mk_file_prop("dds.sec.auth.identity_ca", &self.identity_ca_certificate),
        mk_file_prop(
          "dds.sec.auth.identity_certificate",
          &self.participant_identity_certificate,
        ),
        mk_file_prop(
          "dds.sec.auth.private_key",
          &self.participant_identity_private_key,
        ),
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
        mk_string_prop("dds.sec.auth.password", self.private_key_password),
      ],
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

pub struct SecurityConfig {
  pub(crate) security_enabled: bool,
  pub(crate) properties: qos::policy::Property, // Properties of the DomainParticipantQos
}

pub fn test_config() -> SecurityConfig {
  let security_enabled = true;

  let path_start = [
    "file:".to_string(),
    paths::EXAMPLE_SECURITY_CONFIGURATION_FILES.to_string(),
  ]
  .concat();

  let mut test_participant_name = String::new();
  File::open("test_participant_name")
    .and_then(|mut file| file.read_to_string(&mut test_participant_name))
    .unwrap();

  let properties = vec![
    // For the authentication plugin
    security::types::Property {
      name: "dds.sec.auth.identity_ca".to_string(),
      value: [
        path_start.clone(),
        "identity_ca_certificate.pem".to_string(),
      ]
      .concat(),
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.auth.private_key".to_string(),
      value: [
        path_start.clone(),
        [
          test_participant_name.clone(),
          "_private_key.pem".to_string(),
        ]
        .concat(),
      ]
      .concat(),
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.auth.password".to_string(),
      value: "password123".to_string(), // TODO: Do we need a "data:" prefix here?
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.auth.identity_certificate".to_string(),
      value: [
        path_start.clone(),
        [test_participant_name, "_certificate.pem".to_string()].concat(),
      ]
      .concat(),
      propagate: false,
    },
    // For the access control plugin
    security::types::Property {
      name: "dds.sec.access.permissions_ca".to_string(),
      value: [
        path_start.clone(),
        "permissions_ca_certificate.pem".to_string(),
      ]
      .concat(),
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.access.governance".to_string(),
      value: [path_start.clone(), "test_governance.p7s".to_string()].concat(),
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.access.permissions".to_string(),
      value: [path_start, "test_permissions.p7s".to_string()].concat(),
      propagate: false,
    },
  ];

  let properties = qos::policy::Property {
    value: properties,
    binary_value: vec![],
  };

  SecurityConfig {
    security_enabled,
    properties,
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

use std::{fmt::Debug, fs::File, io::Read};

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
