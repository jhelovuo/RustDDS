use crate::{
  dds::qos::{
    QosPolicies, policy::Deadline, policy::DestinationOrder, policy::Durability, policy::History,
    policy::LatencyBudget, policy::Lifespan, policy::Liveliness, policy::LivelinessKind,
    policy::Ownership, policy::Reliability,
  },
  structure::duration::Duration,
};

pub struct ROSDiscoveryTopic {}

impl ROSDiscoveryTopic {
  const QOS: QosPolicies = QosPolicies {
    durability: Some(Durability::TransientLocal),
    presentation: None,
    deadline: Some(Deadline {
      period: Duration::DURATION_INFINITE,
    }),
    latency_budget: Some(LatencyBudget {
      duration: Duration::DURATION_ZERO,
    }),
    ownership: Some(Ownership::Shared),
    liveliness: Some(Liveliness {
      kind: LivelinessKind::Automatic,
      lease_duration: Duration::DURATION_INFINITE,
    }),
    time_based_filter: None,
    reliability: Some(Reliability::Reliable {
      max_blocking_time: Duration::DURATION_ZERO,
    }),
    destination_order: Some(DestinationOrder::ByReceptionTimestamp),
    history: Some(History::KeepLast { depth: 1 }),
    resource_limits: None,
    lifespan: Some(Lifespan {
      duration: Duration::DURATION_INFINITE,
    }),
  };

  pub fn topic_name() -> String {
    String::from("ros_discovery_info")
  }

  pub fn type_name() -> String {
    String::from("rmw_dds_common::msg::dds_::ParticipantEntitiesInfo_")
  }

  pub fn get_qos() -> QosPolicies {
    ROSDiscoveryTopic::QOS
  }
}
