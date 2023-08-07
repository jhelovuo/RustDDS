use serde::{Deserialize, Serialize};
//use serde_xml_rs::{from_str, to_string};


// Define structs to mirror the XML Schema given in
// DDS Security Spec v1.1 Section
// "9.4.1.2.3 Domain Governance document format"

// TODO: Allow Boolean literals also in all uppercase, e.g. "TRUE" in addition to "true".

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
    pub allow_unauthenticated_participants: bool,
    pub enable_join_access_control: bool,
    pub discovery_protection_kind: ProtectionKind,
    pub liveliness_protection_kind: ProtectionKind,
    pub rtps_protection_kind: ProtectionKind,
    pub topic_access_rules: TopicAccessRules,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DomainIdSet {
    #[serde(rename = "$value")]
    members: Vec<DomainIdSetMember>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum DomainIdSetMember {
  #[serde(rename = "id")]
  DomainId(DomainId),
  #[serde(rename = "id_range")]
  DomainIdRange(DomainIdRange),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DomainId {
  #[serde(rename = "$value")]
  id: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DomainIdRange {
  min: Option<DomainId>, // Both min and max must not be None
  max: Option<DomainId>, 
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProtectionKind {
  EncryptWithOriginAuthentication,
  SignWithOriginAuthentication,
  Encrypt,
  Sign,
  None,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BasicProtectionKind {
  Encrypt,
  Sign,
  None,
}


#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TopicAccessRules {
  #[serde(rename = "$value")]
  pub rules: Vec<TopicRule>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TopicRule {
    pub topic_expression: TopicExpression,
    pub enable_discovery_protection: bool,
    pub enable_liveliness_protection: bool,
    pub enable_read_access_control: bool,
    pub enable_write_access_control: bool,
    pub metadata_protection_kind: ProtectionKind,
    pub data_protection_kind: BasicProtectionKind,
}


#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TopicExpression {
    #[serde(rename = "$value")]
    pub expression: String,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_xml_rs::{from_str};

  #[test]
  pub fn parse_spec_example() {

    // Modifications to example in spec:
    // * insert missing "/" in closing id_range
    // * Boolean literals true/false in all lowercase
    // * field `enable_liveliness_protection` is systematically missing from `topic_rule`s

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
          <enable_read_access_control>false
          </enable_read_access_control>
          <enable_write_access_control>true
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

    let dgd : DomainGovernanceDocument = from_str(domain_governance_document).unwrap();
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

    let dgd : DomainGovernanceDocument = from_str(domain_governance_document).unwrap();
  }


}