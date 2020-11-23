use crate::{
  dds::{participant::*, typedesc::*, qos::*, values::result::*, traits::dds_entity::DDSEntity},
};

pub use crate::structure::topic_kind::TopicKind;

/// Trait estimate of DDS 2.2.2.3.1 TopicDescription Class
pub trait TopicDescription {
  fn get_participant(&self) -> Option<DomainParticipant>;
  fn get_type(&self) -> &TypeDesc; // This replaces get_type_name() from spec
  fn get_name(&self) -> &str;
}

/// DDS Topic
#[derive(Clone)]
pub struct Topic {
  my_domainparticipant: DomainParticipantWeak,
  my_name: String,
  my_typedesc: TypeDesc,
  my_qos_policies: QosPolicies,
  pub topic_kind: TopicKind, // WITH_KEY or NO_KEY
}

impl Topic {
  // visibility pub(crate), because only DomainParticipant should be able to
  // create new Topic objects from an application point of view.
  pub(crate) fn new(
    my_domainparticipant: &DomainParticipantWeak,
    my_name: String,
    my_typedesc: TypeDesc,
    my_qos_policies: &QosPolicies,
    topic_kind: TopicKind,
  ) -> Topic {
    Topic {
      my_domainparticipant: my_domainparticipant.clone(),
      my_name,
      my_typedesc,
      my_qos_policies: my_qos_policies.clone(),
      topic_kind,
    }
  }

  fn get_participant(&self) -> Option<DomainParticipant> {
    self.my_domainparticipant.clone().upgrade()
  }

  fn get_type(&self) -> &TypeDesc {
    &self.my_typedesc
  }

  fn get_name<'a>(&'a self) -> &'a str {
    &self.my_name
  }

  // DDS spec 2.2.2.3.2 Topic Class
  // specifies only method get_inconsistent_topic_status
  // TODO: implement
  pub(crate) fn get_inconsistent_topic_status() -> Result<InconsistentTopicStatus> {
    unimplemented!()
  }
}

/// Implements some default topic interfaces functions defined in DDS spec
impl TopicDescription for Topic {
  /// Gets [DomainParticipant](struct.DomainParticipant.html) if it is still alive.
  fn get_participant(&self) -> Option<DomainParticipant> {
    self.get_participant()
  }

  /// Gets type description of this Topic
  fn get_type(&self) -> &TypeDesc {
    self.get_type()
  }

  /// Gets name of this topic
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
