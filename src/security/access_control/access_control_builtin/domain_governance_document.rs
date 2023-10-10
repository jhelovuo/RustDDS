use serde_xml_rs::from_str;
use glob::*;
pub use xml::{BasicProtectionKind, ProtectionKind};

use crate::security::config::ConfigError;
use super::domain_participant_permissions_document::DomainIds;

// This module provides access (parsing and query) to Domain Governance
// Document as specified in Section "9.4.1.2 Domain Governance Document" of
// DDS Security Spec v1.1

#[derive(Debug)]
pub struct DomainGovernanceDocument {
  domain_access_rules: Vec<DomainRule>,
}

impl DomainGovernanceDocument {
  // Find an applicable rule for domain according to
  // Section "9.4.1.2.7 Application of Domain and Topic Rules"
  //
  // If no rule is found (None), then the operation being attempted must fail with
  // a permissions error.
  pub fn find_rule(&self, domain_id: u16) -> Option<&DomainRule> {
    self.domain_access_rules.iter().find(|dr| {
      dr.domains
        .iter()
        .any(|domain_ids| domain_ids.matches(domain_id))
    })
  }

  pub fn from_xml(xml: &str) -> Result<Self, ConfigError> {
    let dgd: xml::DomainGovernanceDocument = from_str(
      xml
        .trim_start_matches("Content-Type: text/plain")
        .trim_start_matches(char::is_whitespace),
    )?;

    let domain_access_rules = dgd
      .domain_access_rules
      .rule_list
      .iter()
      .map(DomainRule::from_xml)
      .collect::<Result<Vec<DomainRule>, ConfigError>>()?;

    Ok(DomainGovernanceDocument {
      domain_access_rules,
    })
  }
}

#[derive(Debug, Clone)]
pub struct DomainRule {
  pub domains: Vec<DomainIds>,
  pub allow_unauthenticated_participants: bool,
  pub enable_join_access_control: bool,
  pub discovery_protection_kind: ProtectionKind,
  pub liveliness_protection_kind: ProtectionKind,
  pub rtps_protection_kind: ProtectionKind,
  pub topic_access_rules: Vec<TopicRule>,
}

impl DomainRule {
  pub fn find_topic_rule(&self, topic_name: &str) -> Option<&TopicRule> {
    self
      .topic_access_rules
      .iter()
      .find(|tar| tar.topic_expression.matches(topic_name))
  }

  fn from_xml(xr: &xml::DomainRule) -> Result<Self, ConfigError> {
    let domains: Result<Vec<DomainIds>, ConfigError> =
      xr.domains.members.iter().map(DomainIds::from_xml).collect();
    let domains = domains?;

    let topic_access_rules = xr
      .topic_access_rules
      .rules
      .iter()
      .map(TopicRule::from_xml)
      .collect::<Result<Vec<TopicRule>, ConfigError>>()?;

    Ok(DomainRule {
      domains,
      allow_unauthenticated_participants: xr.allow_unauthenticated_participants,
      enable_join_access_control: xr.enable_join_access_control,
      discovery_protection_kind: xr.discovery_protection_kind,
      liveliness_protection_kind: xr.liveliness_protection_kind,
      rtps_protection_kind: xr.rtps_protection_kind,
      topic_access_rules,
    })
  }
}

#[derive(Debug, Clone)]
pub struct TopicRule {
  pub topic_expression: Pattern,
  pub enable_discovery_protection: bool,
  pub enable_liveliness_protection: bool,
  pub enable_read_access_control: bool,
  pub enable_write_access_control: bool,
  pub metadata_protection_kind: ProtectionKind,
  pub data_protection_kind: BasicProtectionKind,
}

impl TopicRule {
  fn from_xml(xtr: &xml::TopicRule) -> Result<Self, ConfigError> {
    let topic_expression =
      Pattern::new(&xtr.topic_expression.expression).map_err(ConfigError::from)?;

    Ok(TopicRule {
      topic_expression,
      enable_discovery_protection: xtr.enable_discovery_protection,
      enable_liveliness_protection: xtr.enable_liveliness_protection,
      enable_read_access_control: xtr.enable_read_access_control,
      enable_write_access_control: xtr.enable_write_access_control,
      metadata_protection_kind: xtr.metadata_protection_kind,
      data_protection_kind: xtr.data_protection_kind,
    })
  }
}

mod xml {
  use serde::{Deserialize, Serialize};

  // Define structs to mirror the XML Schema given in
  // DDS Security Spec v1.1 Section
  // "9.4.1.2.3 Domain Governance document format"

  // The data is structured maybe a bit oddly, because it must mirror the
  // structure of the XSD given in the DDS Security spec.

  #[derive(Debug, Serialize, Deserialize, PartialEq)]
  #[serde(rename = "dds")]
  pub struct DomainGovernanceDocument {
    pub domain_access_rules: DomainAccessRules,
  }

  #[derive(Debug, Serialize, Deserialize, PartialEq)]
  #[serde(rename = "domain_access_rules")]
  pub struct DomainAccessRules {
    #[serde(rename = "$value")]
    pub rule_list: Vec<DomainRule>,
  }

  #[derive(Debug, Serialize, Deserialize, PartialEq)]
  #[serde(rename = "domain_rule")]
  pub struct DomainRule {
    pub domains: DomainIdSet,
    #[serde(deserialize_with = "my_bool_deser")]
    pub allow_unauthenticated_participants: bool,
    #[serde(deserialize_with = "my_bool_deser")]
    pub enable_join_access_control: bool,
    pub discovery_protection_kind: ProtectionKind,
    pub liveliness_protection_kind: ProtectionKind,
    pub rtps_protection_kind: ProtectionKind,
    pub topic_access_rules: TopicAccessRules,
  }

  use super::super::domain_participant_permissions_document::xml::DomainIdSet;

  #[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
  #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
  pub enum ProtectionKind {
    EncryptWithOriginAuthentication,
    SignWithOriginAuthentication,
    Encrypt,
    Sign,
    None,
  }
  impl ProtectionKind {
    /// Outputs ( is_{aspect}_protected, is_{aspect}_encrypted,
    /// is_{aspect}_origin_authenticated ), where {aspect} can be for example
    /// rtps or discovery
    pub(in crate::security::access_control::access_control_builtin) fn to_security_attributes_format(
      self,
    ) -> (bool, bool, bool) {
      match self {
        Self::None => (false, false, false),
        Self::Encrypt => (true, true, false),
        Self::EncryptWithOriginAuthentication => (true, true, true),
        Self::Sign => (true, false, false),
        Self::SignWithOriginAuthentication => (true, false, true),
      }
    }
  }

  #[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
  #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
  pub enum BasicProtectionKind {
    Encrypt,
    Sign,
    None,
  }
  impl BasicProtectionKind {
    /// Outputs ( is_payload_protected, is_payload_encrypted, is_key_protected )
    pub(in crate::security::access_control::access_control_builtin) fn to_security_attributes_format(
      self,
    ) -> (bool, bool, bool) {
      match self {
        Self::None => (false, false, false),
        Self::Encrypt => (true, true, true),
        Self::Sign => (true, false, false),
      }
    }
  }

  #[derive(Debug, Serialize, Deserialize, PartialEq)]
  pub struct TopicAccessRules {
    #[serde(rename = "$value")]
    pub rules: Vec<TopicRule>,
  }

  #[derive(Debug, Serialize, Deserialize, PartialEq)]
  pub struct TopicRule {
    pub topic_expression: TopicExpression,
    #[serde(deserialize_with = "my_bool_deser")]
    pub enable_discovery_protection: bool,
    #[serde(deserialize_with = "my_bool_deser")]
    pub enable_liveliness_protection: bool,
    #[serde(deserialize_with = "my_bool_deser")]
    pub enable_read_access_control: bool,
    #[serde(deserialize_with = "my_bool_deser")]
    pub enable_write_access_control: bool,
    pub metadata_protection_kind: ProtectionKind,
    pub data_protection_kind: BasicProtectionKind,
  }

  #[derive(Debug, Serialize, Deserialize, PartialEq)]
  pub struct TopicExpression {
    #[serde(rename = "$value")]
    pub expression: String,
  }


  // This is a bool adapter, because the built-in
  // serde-xml-rs only accepts lowercase boolean literals.
  use serde::Deserializer;

  fn my_bool_deser<'de, D:Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    String::deserialize(deserializer).and_then(|string| {
      match string.as_str() {
        "false" | "FALSE" | "0" => Ok(false),
        "true" | "TRUE" | "1" => Ok(true),
        other => Err(serde::de::Error::custom(format!("Expected bool: true or false, got {other:?}"))),
      }
    })
  }

} // mod xml

#[cfg(test)]
mod tests {
  use serde_xml_rs::from_str;

  use super::*;

  #[test]
  pub fn parse_spec_example() {
    // Modifications to example in spec:
    // * insert missing "/" in closing id_range
    // * Boolean literals true/false in all lowercase
    // * field `enable_liveliness_protection` is systematically missing from
    //   `topic_rule`s

    let domain_governance_document = r#"<?xml version="1.0" encoding="utf-8"?>
<dds xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
xsi:noNamespaceSchemaLocation="http://www.omg.org/spec/DDS-Security/20170801/omg_shared_ca_domain_governance.xsd">
  <domain_access_rules>
    <domain_rule>
      <domains>
        <id>0</id>
        <id_range>
          <min>10</min>
          <max>20</max>
        </id_range> 
      </domains>
      <allow_unauthenticated_participants>false</allow_unauthenticated_participants>
      <enable_join_access_control>true</enable_join_access_control>
      <rtps_protection_kind>SIGN</rtps_protection_kind>
      <discovery_protection_kind>ENCRYPT</discovery_protection_kind>
      <liveliness_protection_kind>SIGN</liveliness_protection_kind>

      <topic_access_rules>
        <topic_rule>
          <topic_expression>Square*</topic_expression>
          <enable_discovery_protection>true
          </enable_discovery_protection>
          <enable_liveliness_protection>false</enable_liveliness_protection>
          <enable_read_access_control>true
          </enable_read_access_control>
          <enable_write_access_control>true
          </enable_write_access_control>
          <metadata_protection_kind>ENCRYPT
          </metadata_protection_kind>
          <data_protection_kind>ENCRYPT
          </data_protection_kind>
          </topic_rule>
        <topic_rule>
          <topic_expression>Circle</topic_expression>
          <enable_discovery_protection>true
          </enable_discovery_protection>
          <enable_liveliness_protection>false</enable_liveliness_protection>
          <enable_read_access_control>FALSE
          </enable_read_access_control>
          <enable_write_access_control>TRUE
          </enable_write_access_control>
          <metadata_protection_kind>ENCRYPT
          </metadata_protection_kind>
          <data_protection_kind>ENCRYPT
          </data_protection_kind>
        </topic_rule>
        <topic_rule>
          <topic_expression>Triangle
          </topic_expression>
          <enable_discovery_protection>false
          </enable_discovery_protection>
          <enable_liveliness_protection>false</enable_liveliness_protection>
          <enable_read_access_control>false
          </enable_read_access_control>
          <enable_write_access_control>true
          </enable_write_access_control>
          <metadata_protection_kind>NONE
          </metadata_protection_kind>
          <data_protection_kind>NONE
          </data_protection_kind>
        </topic_rule>
        <topic_rule>
          <topic_expression>*</topic_expression>
          <enable_discovery_protection>true
          </enable_discovery_protection>
          <enable_liveliness_protection>false</enable_liveliness_protection>
          <enable_read_access_control>true
          </enable_read_access_control>
          <enable_write_access_control>true
          </enable_write_access_control>
          <metadata_protection_kind>ENCRYPT
          </metadata_protection_kind>
          <data_protection_kind>ENCRYPT
          </data_protection_kind>
        </topic_rule>
      </topic_access_rules>
    </domain_rule>
  </domain_access_rules>
</dds>
"#;

    from_str::<xml::DomainGovernanceDocument>(domain_governance_document).unwrap();

    println!(
      "{:?}",
      DomainGovernanceDocument::from_xml(domain_governance_document).unwrap()
    );
  }

  #[test]
  pub fn parse_minimal() {
    let domain_governance_document = r#"<?xml version="1.0" encoding="utf-8"?>
<dds xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
xsi:noNamespaceSchemaLocation="http://www.omg.org/spec/DDS-Security/20170801/omg_shared_ca_domain_governance.xsd">
  <domain_access_rules>
    <domain_rule>
      <domains>
        <id>0</id>
      </domains>

      <allow_unauthenticated_participants>false</allow_unauthenticated_participants>
      <enable_join_access_control>true</enable_join_access_control>
      <rtps_protection_kind>SIGN</rtps_protection_kind>
      <discovery_protection_kind>SIGN</discovery_protection_kind>
      <liveliness_protection_kind>SIGN</liveliness_protection_kind>

      <topic_access_rules>
        <topic_rule>
          <topic_expression>Square*</topic_expression>
          <enable_discovery_protection>true
          </enable_discovery_protection>
          <enable_liveliness_protection>false</enable_liveliness_protection>
          <enable_read_access_control>true
          </enable_read_access_control>
          <enable_write_access_control>TRUE
          </enable_write_access_control>
          <metadata_protection_kind>ENCRYPT
          </metadata_protection_kind>
          <data_protection_kind>ENCRYPT
          </data_protection_kind>
        </topic_rule>
      </topic_access_rules>
    </domain_rule>
  </domain_access_rules>
</dds>
"#;

    from_str::<xml::DomainGovernanceDocument>(domain_governance_document).unwrap();

    println!(
      "{:?}",
      DomainGovernanceDocument::from_xml(domain_governance_document).unwrap()
    );
  }
}
