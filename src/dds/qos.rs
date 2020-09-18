use crate::dds::values::result::*;

// This is to be implemented by all DomanParticipant, Publisher, Subscriber, DataWriter, DataReader, Topic
pub trait HasQoSPolicy {
  fn get_qos(&self) -> &QosPolicies;
  fn set_qos(&mut self, new_qos: &QosPolicies) -> Result<()>;
}

// DDS spec 2.3.3 defines this as "long" with named constants from 0 to 22.
// numbering is from IDL PSM, but it should be unnecessary at the Rust application interface
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum QosPolicyId {
  //Invalid  // We should represent this using Option<QosPolicyId> where needed
  //UserData,  // 1
  Durability,   // 2
  Presentation, // 3
  Deadline,
  LatencyBudget, // 5
  Ownership,
  //OwnershipStrength, // 7
  Liveliness,
  TimeBasedFilter, // 9
  //Partition,
  Reliability, // 11
  DestinationOrder,
  History, // 13
  ResourceLimits,
  //EntityFactory, // 15
  //WriterDataLifeCycle,
  //ReaderDataLifeCycle, // 17
  //TopicData, // 18
  //GroupData,
  //TransportPriority, // 20
  Lifespan,
  //DurabilityService, // 22
}

#[derive(Clone, Debug)]
pub struct QosPolicies {
  pub durability: Option<policy::Durability>,
  pub presentation: Option<policy::Presentation>,
  pub deadline: Option<policy::Deadline>,
  pub latency_budget: Option<policy::LatencyBudget>,
  pub ownership: Option<policy::Ownership>,
  pub liveliness: Option<policy::Liveliness>,
  pub time_based_filter: Option<policy::TimeBasedFilter>,
  pub reliability: Option<policy::Reliability>,
  pub destination_order: Option<policy::DestinationOrder>,
  pub history: Option<policy::History>,
  pub resource_limits: Option<policy::ResourceLimits>,
  pub lifespan: Option<policy::Lifespan>,
}

impl QosPolicies {
  pub fn qos_none() -> QosPolicies {
    QosPolicies {
      durability: None,
      presentation: None,
      deadline: None,
      latency_budget: None,
      ownership: None,
      liveliness: None,
      time_based_filter: None,
      reliability: None,
      destination_order: None,
      history: None,
      resource_limits: None,
      lifespan: None,
    }
  }

  pub fn history(&self) -> &Option<policy::History> {
    &self.history
  }
}

// put these into a submodule to avoid repeating the word "policy" or "qospolicy"
pub mod policy {
  use crate::structure::{parameter_id::ParameterId, duration::Duration};
  use serde::{Serialize, Deserialize};

  /*
  pub struct UserData {
    pub value: Vec<u8>,
  }

  pub struct TopicData {
    pub value: Vec<u8>,
  }

  pub struct GropupData {
    pub value: Vec<u8>,
  }

  pub struct TransportPriority {
    pub value: i32,
  }
  */
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct Lifespan {
    pub duration: Duration,
  }

  // this is a policy
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub enum Durability {
    Volatile,
    TransientLocal,
    Transient,
    Persistent,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct Presentation {
    pub access_scope: PresentationAccessScope,
    pub coherent_access: bool,
    pub ordered_access: bool,
  }

  // This is not an independent QoS Policy but a component of Presentation
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub enum PresentationAccessScope {
    Instance,
    Topic,
    Group,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct Deadline {
    pub period: Duration,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct LatencyBudget {
    pub duration: Duration,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub enum Ownership {
    Shared,
    Exclusive { strength: i32 }, // This also implements OwnershipStrength
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct Liveliness {
    pub kind: LivelinessKind,
    pub lease_duration: Duration,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub enum LivelinessKind {
    Automatic,
    ManualByParticipant,
    ManulByTopic,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct TimeBasedFilter {
    pub minimum_separation: Duration,
  }

  /*
  pub struct Partition {
    pub name: Vec<Vec<u8>>,
  }
  */

  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub enum Reliability {
    BestEffort,
    Reliable { max_blocking_time: Duration },
  }

  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub enum DestinationOrder {
    ByReceptionTimestamp,
    BySourceTimeStamp,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub enum History {
    KeepLast { depth: i32 },
    KeepAll,
  }

  #[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct ResourceLimits {
    pub max_samples: i32,
    pub max_instances: i32,
    pub max_samples_per_instance: i32,
  }

  #[derive(Serialize, Deserialize)]
  pub struct QosData<D>
  where
    D: Serialize,
  {
    parameter_id: ParameterId,
    parameter_length: u16,
    qos_param: D,
  }

  impl<D> QosData<D>
  where
    D: Serialize + Copy + Clone,
  {
    pub fn new(parameter_id: ParameterId, qosparam: D) -> QosData<D> {
      match parameter_id {
        ParameterId::PID_DURABILITY => QosData {
          parameter_id,
          parameter_length: 4,
          qos_param: qosparam.clone(),
        },
        ParameterId::PID_DEADLINE
        | ParameterId::PID_LATENCY_BUDGET
        | ParameterId::PID_TIME_BASED_FILTER
        | ParameterId::PID_PRESENTATION
        | ParameterId::PID_LIFESPAN
        | ParameterId::PID_HISTORY => QosData {
          parameter_id,
          parameter_length: 8,
          qos_param: qosparam.clone(),
        },
        ParameterId::PID_LIVELINESS
        | ParameterId::PID_RELIABILITY
        | ParameterId::PID_RESOURCE_LIMITS => QosData {
          parameter_id,
          parameter_length: 12,
          qos_param: qosparam.clone(),
        },
        _ => QosData {
          parameter_id,
          parameter_length: 4,
          qos_param: qosparam.clone(),
        },
      }
    }
  }

  /*
  pub struct EntityFactory {
    autoenable_created_entities: bool,
  }
  */
  // WriterDataLifecycle
  // ReaderDataLifeCycle

  // DurabilityService
}

// TODO: helper function to combine two QosPolicies: existing and modifications
// Described in 2.2.2.1.1.1 set_qos (abstract)

// TODO: helper function to check is a QosPolices object is inconsistent (by itself)

// TODO: helper function to check if two QosPolicies: Reequested and Offered are
// compatible, according to DDS spec 2.2.3
