use serde::{Deserialize, Serialize};
//use serde_xml_rs::{from_str, to_string};

// Define structs to mirror the XML Schema given in
// DDS Security Spec v1.1 Section
// "9.4.1.3 DomainParticipant permissions document"

// TODO: Allow Boolean literals also in all uppercase, e.g. "TRUE" in addition
// to "true".

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "dds")]
pub struct DomainParticiapntPermissionsDocument {
  pub permissions: Permissions,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Permissions {
  #[serde(rename = "$value")]
  pub grants: Vec<Grant>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "grant")]
pub struct Grant {
  #[serde(rename = "$value")]
  pub elems: Vec<GrantElement>,
  //TODO: This is a hacky way to get serde-xml to read the XML as specified.
  // We need to manually check that there is (exactly) one of each SubjectName, Validity, and
  // Default in a Grant.
  // There may be an arbitray number of AllowRules and DenyRules, and their order is important.
  // The AllowRules and DenyRules are to scanned in order until one matches, and that is to be
  // applied. if there is no match, then use Default.
  //
  // See Section "9.4.1.3.2.3 Rules Section" in DDS Security Spec v1.1
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GrantElement {
  SubjectName(String), // This is a X.509 subject name, so may need further parsing
  Validity(Validity),
  AllowRule(Rule),
  DenyRule(Rule),
  Default(DefaultAction),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Validity {
  not_before: String, // XsdDateTime,
  not_after: String,  // XsdDateTime,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Rule {
  #[serde(rename = "$value")]
  pub elems: Vec<RuleElement>, /*TODO: This is a hacky way to get serde-xml to read the XML as
                                * specified. We need to manually
                                * check that there is (exactly) one of `domain`
                                * in a Rule, breferably at the begining. */
}

// The RuleElements should be in order Publish, Subscribe, Relay, witch 0..N
// occurencences of each. This definition accepts them in any order.

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RuleElement {
  Domains(DomainIdSet),
  Publish(Criteria),
  Subscribe(Criteria),
  Relay(Criteria),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DomainIdSet {
  #[serde(rename = "$value")]
  pub members: Vec<DomainIdSetMember>,
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
  pub id: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DomainIdRange {
  pub min: Option<DomainId>, // At least one of these must be defined.
  pub max: Option<DomainId>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Criteria {
  #[serde(rename = "$value")]
  pub members: Vec<Criterion>, // must not be empty: must have at least 1 topic criterion specified
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Criterion {
  Topics(TopicExpressionList),
  Partitions(PartitionExpressionList),
  DataTags(DataTags),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TopicExpressionList {
  #[serde(rename = "$value")]
  pub members: TopicExpression,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TopicExpression {
  #[serde(rename = "$value")]
  pub value: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PartitionExpressionList {
  #[serde(rename = "$value")]
  pub members: Vec<PartitionExpression>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PartitionExpression {
  #[serde(rename = "$value")]
  pub value: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DataTags {
  #[serde(rename = "$value")]
  pub members: Vec<DataTag>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "tag")]
pub struct DataTag {
  pub name: String,
  pub value: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DefaultAction {
  Allow,
  Deny,
}

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

    let domain_governance_document = r#"<?xml version="1.0" encoding="UTF-8"?>
<dds xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:noNamespaceSchemaLocation="http://www.omg.org/spec/DDS-Security/20170801/omg_shared_ca_permissions.xsd">
  <permissions>
    <grant name="ShapesPermission">
      <subject_name>emailAddress=cto@acme.com, CN=DDS Shapes Demo, OU=CTO Office, O=ACME Inc., L=Sunnyvale, ST=CA, C=US</subject_name>
      <validity>
        <!-- Format is CCYY-MM-DDThh:mm:ss[Z|(+|-)hh:mm] The time zone may
        be specified as Z (UTC) or (+|-)hh:mm. Time zones that aren't
        specified are considered UTC.
        -->
        <not_before>2013-10-26T00:00:00</not_before>
        <not_after>2018-10-26T22:45:30</not_after>
      </validity>

      <allow_rule>
        <domains>
        <id>0</id>
        </domains>
        <!-- DDSSEC11-56 - deleted invalid elements -->
      </allow_rule>

      <deny_rule>
        <domains>
          <id>0</id>
        </domains>
        <publish>
          <topics>
          <topic>Circle1</topic>
          </topics>
        </publish>
        <publish>
          <topics>
          <topic>Square</topic>
          </topics>
          <partitions>
          <partition>A_partition</partition>
          </partitions>
        </publish>
        <subscribe>
          <topics>
          <topic>Square1</topic>
          </topics>
        </subscribe>
        <subscribe>
          <topics>
          <topic>Tr*</topic>
          </topics>
          <partitions>
          <partition>P1*</partition>
          </partitions>
        </subscribe>
      </deny_rule>

      <allow_rule>
        <domains>
        <id>0</id>
        </domains>
        <publish>
        <topics>
        <topic>Cir*</topic>
        </topics>
        <data_tags>
        <tag>
        <name>aTagName1</name>
        <value>aTagValue1</value>
        </tag>
        </data_tags>
        </publish>
        <subscribe>
        <topics>
        <topic>Sq*</topic>
        </topics>
        <data_tags>
        <tag>
        <name>aTagName1</name>
        <value>aTagValue1</value>
        </tag>
        <tag>
        <name>aTagName2</name>
        <value>aTagValue2</value>
        </tag>
        </data_tags>
        </subscribe>
        <subscribe>
        <topics>
        <topic>Triangle</topic>
        </topics>
        <partitions>
        <partition>P*</partition>
        </partitions>
        <data_tags>
        <tag>
        <name>aTagName1</name>
        <value>aTagValue1</value>
        </tag>
        </data_tags>
        </subscribe>
        <relay>
        <topics>
        <topic>*</topic>
        </topics>
        <partitions>
        <partition>aPartitionName</partition>
        </partitions>
        </relay>
      </allow_rule>

      <default>DENY</default>

    </grant>
  </permissions>
</dds>
"#;

    let dgd: DomainParticiapntPermissionsDocument = from_str(domain_governance_document).unwrap();
  }

  #[test]
  pub fn parse_minimal() {
    let domain_governance_document = r#"<?xml version="1.0" encoding="UTF-8"?>
<dds xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:noNamespaceSchemaLocation="http://www.omg.org/spec/DDS-Security/20170801/omg_shared_ca_permissions.xsd">
  <permissions>
    <grant name="ShapesPermission">
      <subject_name>emailAddress=cto@acme.com, CN=DDS Shapes Demo, OU=CTO Office, O=ACME Inc., L=Sunnyvale, ST=CA, C=US</subject_name>
      <validity>
        <not_before>2013-10-26T00:00:00</not_before>
        <not_after>2018-10-26T22:45:30</not_after>
      </validity>

      <allow_rule>
        <domains><id> 0 </id></domains>
      </allow_rule>

      <default>DENY</default>

    </grant>
  </permissions>
</dds>
"#;

    let dgd: DomainParticiapntPermissionsDocument = from_str(domain_governance_document).unwrap();
  }
}
