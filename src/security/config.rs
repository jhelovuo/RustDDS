pub mod paths;
use crate::{qos, security};

// Temporary module for determining security configurations for development
// purposes.

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
        "participant1_private_key.pem".to_string(),
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
        "participant1_certificate.pem".to_string(),
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
      value: [path_start.clone(), "permissive_governance.p7s".to_string()].concat(),
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.access.permissions".to_string(),
      value: [path_start, "permissive_permissions.p7s".to_string()].concat(),
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
