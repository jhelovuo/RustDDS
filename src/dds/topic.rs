use std::{fmt::Debug, sync::Arc};

use crate::dds::{participant::*, qos::*, traits::dds_entity::DDSEntity, typedesc::*};
pub use crate::structure::topic_kind::TopicKind;

/// Trait estimate of DDS 2.2.2.3.1 TopicDescription Class
pub trait TopicDescription {
  fn get_participant(&self) -> Option<DomainParticipant>;
  fn get_type(&self) -> TypeDesc; // This replaces get_type_name() from spec
  fn get_name(&self) -> String;
}

/// DDS Topic
///
/// # Examples
///
/// ```
/// use rustdds::dds::DomainParticipant;
/// use rustdds::dds::qos::QosPolicyBuilder;
/// use rustdds::dds::Topic;
/// use rustdds::dds::data_types::TopicKind;
///
/// let domain_participant = DomainParticipant::new(0).unwrap();
/// let qos = QosPolicyBuilder::new().build();
/// let topic = domain_participant
///       .create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey)
///       .unwrap();
/// ```
#[derive(Clone)]
pub struct Topic {
  //TODO: Do we really need set_qos operation?
  // Maybe not. Let's make Topic immutable.
  inner: Arc<InnerTopic>,
}

impl Topic {
  pub(crate) fn new(
    my_domainparticipant: &DomainParticipantWeak,
    my_name: String,
    my_typedesc: TypeDesc,
    my_qos_policies: &QosPolicies,
    topic_kind: TopicKind,
  ) -> Topic {
    Topic {
      inner: Arc::new(InnerTopic::new(
        my_domainparticipant,
        my_name,
        my_typedesc,
        my_qos_policies,
        topic_kind,
      )),
    }
  }

  fn get_participant(&self) -> Option<DomainParticipant> {
    self.inner.get_participant()
  }

  // TODO: Confusing combination of borrows and owns
  fn get_type(&self) -> TypeDesc {
    self.inner.get_type()
  }

  fn get_name(&self) -> String {
    self.inner.get_name()
  }

  /// Gets Topics TopicKind
  ///
  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::Topic;
  /// use rustdds::dds::data_types::TopicKind;
  ///
  /// # let domain_participant = DomainParticipant::new(0).unwrap();
  /// # let qos = QosPolicyBuilder::new().build();
  /// let topic = domain_participant
  ///     .create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey)
  ///     .unwrap();
  /// assert_eq!(topic.kind(), TopicKind::WithKey);
  /// ```
  pub fn kind(&self) -> TopicKind {
    self.inner.kind()
  }
  /*
  // DDS spec 2.2.2.3.2 Topic Class
  // specifies only method get_inconsistent_topic_status
  // TODO: implement
  pub(crate) fn get_inconsistent_topic_status() -> Result<InconsistentTopicStatus> {
    unimplemented!()
  }
  */
}

impl PartialEq for Topic {
  fn eq(&self, other: &Self) -> bool {
    self.inner == other.inner
  }
}

impl Debug for Topic {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.inner.fmt(f)
  }
}

/// Implements some default topic interfaces functions defined in DDS spec
impl TopicDescription for Topic {
  /// Gets [DomainParticipant](struct.DomainParticipant.html) if it is still
  /// alive.
  fn get_participant(&self) -> Option<DomainParticipant> {
    self.get_participant()
  }

  /// Gets type description of this Topic
  fn get_type(&self) -> TypeDesc {
    self.get_type()
  }

  /// Gets name of this topic
  fn get_name(&self) -> String {
    self.get_name()
  }
}

impl HasQoSPolicy for Topic {
  fn get_qos(&self) -> QosPolicies {
    self.inner.get_qos()
  }
}

//impl DDSEntity for Topic {}

// -------------------------------- InnerTopic -----------------------------

#[derive(Clone)]
pub struct InnerTopic {
  my_domainparticipant: DomainParticipantWeak,
  my_name: String,
  my_typedesc: TypeDesc,
  my_qos_policies: QosPolicies,
  topic_kind: TopicKind, // WITH_KEY or NO_KEY
}

impl InnerTopic {
  // visibility pub(crate), because only DomainParticipant should be able to
  // create new Topic objects from an application point of view.
  fn new(
    my_domainparticipant: &DomainParticipantWeak,
    my_name: String,
    my_typedesc: TypeDesc,
    my_qos_policies: &QosPolicies,
    topic_kind: TopicKind,
  ) -> InnerTopic {
    InnerTopic {
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

  fn get_type(&self) -> TypeDesc {
    self.my_typedesc.clone()
  }

  fn get_name(&self) -> String {
    self.my_name.to_string()
  }

  pub fn kind(&self) -> TopicKind {
    self.topic_kind
  }
  /*
  pub(crate) fn get_inconsistent_topic_status() -> Result<TopicStatus> {
    unimplemented!()
  } */
}

impl PartialEq for InnerTopic {
  fn eq(&self, other: &Self) -> bool {
    self.get_participant() == other.get_participant()
      && self.get_type() == other.get_type()
      && self.get_name() == other.get_name()
      && self.get_qos() == other.get_qos()
      && self.topic_kind == other.topic_kind
  }
}

impl Debug for InnerTopic {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("{:?}", self.get_participant()))?;
    f.write_fmt(format_args!("Topic name: {}", self.get_name()))?;
    f.write_fmt(format_args!("Topic type: {:?}", self.get_type()))?;
    f.write_fmt(format_args!("Topic QoS: {:?} ", self.get_qos()))
  }
}

/// Implements some default topic interfaces functions defined in DDS spec
impl TopicDescription for InnerTopic {
  /// Gets [DomainParticipant](struct.DomainParticipant.html) if it is still
  /// alive.
  fn get_participant(&self) -> Option<DomainParticipant> {
    self.get_participant()
  }

  /// Gets type description of this Topic
  fn get_type(&self) -> TypeDesc {
    self.get_type()
  }

  /// Gets name of this topic
  fn get_name(&self) -> String {
    self.get_name()
  }
}

impl HasQoSPolicy for InnerTopic {
  // fn set_qos(&mut self, policy: &QosPolicies) -> Result<()> {
  //   // TODO: check liveliness of qos_polic
  //   self.my_qos_policies = policy.clone();
  //   Ok(())
  // }

  fn get_qos(&self) -> QosPolicies {
    self.my_qos_policies.clone()
  }
}

impl DDSEntity for InnerTopic {}
