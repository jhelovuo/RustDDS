use crate::dds::{
  participant::*, typedesc::*, qos::*, values::result::*, traits::dds_entity::DDSEntity,
};

pub trait TopicDescription {
  fn get_participant(&self) -> &DomainParticipant;
  fn get_type(&self) -> &TypeDesc; // This replaces get_type_name() from spec
  fn get_name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct Topic {
  my_domainparticipant: DomainParticipant,
  my_name: String,
  my_typedesc: TypeDesc,
  my_qos_policies: QosPolicies,
}

impl Topic {
  // visibility pub(crate), because only DomainParticipant should be able to
  // create new Topic objects from an application point of view.
  pub(crate) fn new(
    my_domainparticipant: &DomainParticipant,
    my_name: String,
    my_typedesc: TypeDesc,
    my_qos_policies: &QosPolicies,
  ) -> Topic {
    Topic {
      my_domainparticipant: my_domainparticipant.clone(),
      my_name,
      my_typedesc,
      my_qos_policies: my_qos_policies.clone(),
    }
  }

  pub fn get_participant(&self) -> &DomainParticipant {
    &self.my_domainparticipant
  }

  pub fn get_type(&self) -> &TypeDesc {
    &self.my_typedesc
  }

  pub fn get_name(&self) -> &str {
    &self.my_name
  }

  // DDS spec 2.2.2.3.2 Topic Class
  // specifies only method get_inconsistent_topic_status
  pub fn get_inconsistent_topic_status() -> Result<InconsistentTopicStatus> {
    unimplemented!()
  }
}

impl TopicDescription for Topic {
  fn get_participant(&self) -> &DomainParticipant {
    self.get_participant()
  }

  fn get_type(&self) -> &TypeDesc {
    self.get_type()
  }

  fn get_name(&self) -> &str {
    self.get_name()
  }
}

impl HasQoSPolicy for Topic {
  fn set_qos(&mut self, policy: &QosPolicies) -> Result<()> {
    // TODO: check liveliness of qos_polic
    self.my_qos_policies = policy.clone();
    Ok(())
  }

  fn get_qos(&self) -> &QosPolicies {
    &self.my_qos_policies
  }
}

impl DDSEntity for Topic {}
