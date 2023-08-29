use crate::{qos, security};

// Temporary module for determining security configurations for development
// purposes.

pub struct SecurityConfig {
  pub(crate) security_enabled: bool,
  pub(crate) properties: qos::policy::Property, // Properties of the DomainParticipantQos
}

pub fn test_config() -> SecurityConfig {
  let security_enabled = true;

  let properties = vec![
    // For the authentication plugin
    security::types::Property {
      name: "dds.sec.auth.identity_ca".to_string(),
      value: "file:identity_ca.pem".to_string(),
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.auth.private_key".to_string(),
      value: "file:identity_ca_private_key.pem".to_string(),
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.auth.password".to_string(),
      value: "passwd".to_string(),
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.auth.identity_certificate".to_string(),
      value: "file:participant1_identity_cert.pem".to_string(),
      propagate: false,
    },
    // For the access control plugin
    security::types::Property {
      name: "dds.sec.access.permissions_ca".to_string(),
      value: "file:example_security_configuration_files/permissions_ca.pem".to_string(),
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.access.governance".to_string(),
      value: "file:example_security_configuration_files/permissive_governance.p7s".to_string(),
      propagate: false,
    },
    security::types::Property {
      name: "dds.sec.access.permissions".to_string(),
      value: "file:example_security_configuration_files/permissive_permissions.p7s".to_string(),
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
