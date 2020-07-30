//use std::time::Duration;

use crate::dds::participant::*;
//use crate::dds::key::*;
use crate::dds::typedesc::*;
use crate::dds::qos::*;
use crate::dds::values::result::*;
use crate::dds::traits::dds_entity::DDSEntity;

pub trait TopicDescription {
  fn get_participant(&self) -> &DomainParticipant;
  fn get_type(&self) -> TypeDesc; // This replaces get_type_name() from spec
  fn get_name(&self) -> &str;
}
#[derive(Debug)]
pub struct Topic<'a> {
  my_domainparticipant: &'a DomainParticipant,
  my_name: String,
  my_typedesc: TypeDesc,
  my_qos_policies: QosPolicies,
}

impl<'a> Topic<'a> {
  pub fn new(
    my_domainparticipant: &'a DomainParticipant,
    my_name: String,
    my_typedesc: TypeDesc,
    my_qos_policies: QosPolicies,
  ) -> Topic<'a> {
    Topic {
      my_domainparticipant,
      my_name,
      my_typedesc,
      my_qos_policies,
    }
  }
}

impl<'a> TopicDescription for Topic<'a> {
  fn get_participant(&self) -> &DomainParticipant {
    self.my_domainparticipant
  }

  fn get_type(&self) -> TypeDesc {
    self.my_typedesc.clone()
  }

  fn get_name(&self) -> &str {
    &self.my_name
  }
}

impl<'a> Topic<'a> {
  // DDS spec 2.2.2.3.2 Topic Class
  // specifies only method get_inconsistent_topic_status
  pub fn get_inconsistent_topic_status() -> Result<InconsistentTopicStatus> {
    unimplemented!()
  }
}

impl<'a> HasQoSPolicy<'a> for Topic<'a> {
  fn set_qos(mut self, policy: &QosPolicies) -> Result<()> {
    // TODO: check liveliness of qos_policy
    self.my_qos_policies = policy.clone();
    Ok(())
  }

  fn get_qos(&'a self) -> &'a QosPolicies {
    &self.my_qos_policies
  }
}

impl<'a> DDSEntity<'a> for Topic<'a> {}
