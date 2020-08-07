use crate::dds::{
  participant::*, typedesc::*, qos::*, values::result::*, traits::dds_entity::DDSEntity,
};

use std::sync::Arc;

pub trait TopicDescription {
  fn get_participant(&self) -> Arc<DomainParticipant>;
  fn get_type(&self) -> TypeDesc; // This replaces get_type_name() from spec
  fn get_name(&self) -> &str;
}
#[derive(Debug)]
pub struct Topic {
  my_domainparticipant: Arc<DomainParticipant>,
  my_name: String,
  my_typedesc: TypeDesc,
  my_qos_policies: QosPolicies,
}

impl Topic {
  pub fn new(
    my_domainparticipant: Arc<DomainParticipant>,
    my_name: String,
    my_typedesc: TypeDesc,
    my_qos_policies: QosPolicies,
  ) -> Topic {
    Topic {
      my_domainparticipant,
      my_name,
      my_typedesc,
      my_qos_policies,
    }
  }
}

impl TopicDescription for Topic {
  fn get_participant(&self) -> Arc<DomainParticipant> {
    self.my_domainparticipant.clone()
  }

  fn get_type(&self) -> TypeDesc {
    self.my_typedesc.clone()
  }

  fn get_name(&self) -> &str {
    &self.my_name
  }
}

impl Topic {
  // DDS spec 2.2.2.3.2 Topic Class
  // specifies only method get_inconsistent_topic_status
  pub fn get_inconsistent_topic_status() -> Result<InconsistentTopicStatus> {
    unimplemented!()
  }
}

impl HasQoSPolicy for Topic {
  fn set_qos(mut self, policy: &QosPolicies) -> Result<()> {
    // TODO: check liveliness of qos_policy
    self.my_qos_policies = policy.clone();
    Ok(())
  }

  fn get_qos(&self) -> &QosPolicies {
    &self.my_qos_policies
  }
}

impl DDSEntity for Topic {}
