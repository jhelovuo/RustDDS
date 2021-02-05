use log::{trace,};

use crate::{
  structure::inline_qos::KeyHash,
  dds::values::result::*,
  messages::submessages::submessage_elements::{
    parameter_list::ParameterList, RepresentationIdentifier,
  },
  structure::{parameter_id::ParameterId, inline_qos::StatusInfo},
};

// This is to be implemented by all DomanParticipant, Publisher, Subscriber, DataWriter, DataReader, Topic
/// Trait that is implemented by all necessary DDS Entities that are required to provide QosPolicies.
pub trait HasQoSPolicy {
  fn get_qos(&self) -> QosPolicies;
}

/// Trait that is implemented by all necessary DDS Entities that are required to have a mutable QosPolicies.
pub trait MutQosPolicy {
  fn set_qos(&mut self, new_qos: &QosPolicies) -> Result<()>;
}

/// DDS spec 2.3.3 defines this as "long" with named constants from 0 to 22.
/// numbering is from IDL PSM, but it should be unnecessary at the Rust application interface
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

/// Utility for building [QosPolicies](struct.QosPolicies.html)
pub struct QosPolicyBuilder {
  durability: Option<policy::Durability>,
  presentation: Option<policy::Presentation>,
  deadline: Option<policy::Deadline>,
  latency_budget: Option<policy::LatencyBudget>,
  ownership: Option<policy::Ownership>,
  liveliness: Option<policy::Liveliness>,
  time_based_filter: Option<policy::TimeBasedFilter>,
  reliability: Option<policy::Reliability>,
  destination_order: Option<policy::DestinationOrder>,
  history: Option<policy::History>,
  resource_limits: Option<policy::ResourceLimits>,
  lifespan: Option<policy::Lifespan>,
}

impl QosPolicyBuilder {
  pub const fn new() -> QosPolicyBuilder {
    QosPolicyBuilder {
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

  pub const fn durability(mut self, durability: policy::Durability) -> QosPolicyBuilder {
    self.durability = Some(durability);
    self
  }

  pub const fn presentation(mut self, presentation: policy::Presentation) -> QosPolicyBuilder {
    self.presentation = Some(presentation);
    self
  }

  pub const fn deadline(mut self, deadline: policy::Deadline) -> QosPolicyBuilder {
    self.deadline = Some(deadline);
    self
  }

  pub const fn latency_budget(mut self, latency_budget: policy::LatencyBudget) -> QosPolicyBuilder {
    self.latency_budget = Some(latency_budget);
    self
  }

  pub const fn ownership(mut self, ownership: policy::Ownership) -> QosPolicyBuilder {
    self.ownership = Some(ownership);
    self
  }

  pub const fn liveliness(mut self, liveliness: policy::Liveliness) -> QosPolicyBuilder {
    self.liveliness = Some(liveliness);
    self
  }

  pub const fn time_based_filter(
    mut self,
    time_based_filter: policy::TimeBasedFilter,
  ) -> QosPolicyBuilder {
    self.time_based_filter = Some(time_based_filter);
    self
  }

  pub const fn reliability(mut self, reliability: policy::Reliability) -> QosPolicyBuilder {
    self.reliability = Some(reliability);
    self
  }

  pub const fn destination_order(
    mut self,
    destination_order: policy::DestinationOrder,
  ) -> QosPolicyBuilder {
    self.destination_order = Some(destination_order);
    self
  }

  pub const fn history(mut self, history: policy::History) -> QosPolicyBuilder {
    self.history = Some(history);
    self
  }

  pub const fn resource_limits(
    mut self,
    resource_limits: policy::ResourceLimits,
  ) -> QosPolicyBuilder {
    self.resource_limits = Some(resource_limits);
    self
  }

  pub const fn lifespan(mut self, lifespan: policy::Lifespan) -> QosPolicyBuilder {
    self.lifespan = Some(lifespan);
    self
  }

  pub const fn build(self) -> QosPolicies {
    QosPolicies {
      durability: self.durability,
      presentation: self.presentation,
      deadline: self.deadline,
      latency_budget: self.latency_budget,
      ownership: self.ownership,
      liveliness: self.liveliness,
      time_based_filter: self.time_based_filter,
      reliability: self.reliability,
      destination_order: self.destination_order,
      history: self.history,
      resource_limits: self.resource_limits,
      lifespan: self.lifespan,
    }
  }
}

/// Describes single RTPS/DDS QoS policy
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QosPolicies {
  // pub(crate) beacuse as we want to have some builtin QoS Policies as constant.
  pub(crate) durability: Option<policy::Durability>,
  pub(crate) presentation: Option<policy::Presentation>,
  pub(crate) deadline: Option<policy::Deadline>,
  pub(crate) latency_budget: Option<policy::LatencyBudget>,
  pub(crate) ownership: Option<policy::Ownership>,
  pub(crate) liveliness: Option<policy::Liveliness>,
  pub(crate) time_based_filter: Option<policy::TimeBasedFilter>,
  pub(crate) reliability: Option<policy::Reliability>,
  pub(crate) destination_order: Option<policy::DestinationOrder>,
  pub(crate) history: Option<policy::History>,
  pub(crate) resource_limits: Option<policy::ResourceLimits>,
  pub(crate) lifespan: Option<policy::Lifespan>,
}

impl QosPolicies {
  #[cfg(test)]
  pub const fn qos_none() -> QosPolicies {
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

  pub const fn builder() -> QosPolicyBuilder {
    QosPolicyBuilder::new()
  }

  pub const fn durability(&self) -> Option<policy::Durability> {
    self.durability
  }

  pub const fn presentation(&self) -> Option<policy::Presentation> {
    self.presentation
  }

  pub const fn deadline(&self) -> Option<policy::Deadline> {
    self.deadline
  }

  pub const fn latency_budget(&self) -> Option<policy::LatencyBudget> {
    self.latency_budget
  }

  pub const fn ownership(&self) -> Option<policy::Ownership> {
    self.ownership
  }

  pub const fn liveliness(&self) -> Option<policy::Liveliness> {
    self.liveliness
  }

  pub const fn time_based_filter(&self) -> Option<policy::TimeBasedFilter> {
    self.time_based_filter
  }

  pub const fn reliability(&self) -> Option<policy::Reliability> {
    self.reliability
  }

  pub const fn destination_order(&self) -> Option<policy::DestinationOrder> {
    self.destination_order
  }

  pub const fn history(&self) -> Option<policy::History> {
    self.history
  }

  pub const fn resource_limits(&self) -> Option<policy::ResourceLimits> {
    self.resource_limits
  }

  pub const fn lifespan(&self) -> Option<policy::Lifespan> {
    self.lifespan
  }

  pub fn modify_by(&self,other: &QosPolicies) -> QosPolicies {
    QosPolicies {
      durability: other.durability.or(self.durability),
      presentation: other.presentation.or(self.presentation),
      deadline: other.deadline.or(self.deadline),
      latency_budget: other.latency_budget.or(self.latency_budget),
      ownership: other.ownership.or(self.ownership),
      liveliness: other.liveliness.or(self.liveliness),
      time_based_filter: other.time_based_filter.or(self.time_based_filter),
      reliability: other.reliability.or(self.reliability),
      destination_order: other.destination_order.or(self.destination_order),
      history: other.history.or(self.history),
      resource_limits: other.resource_limits.or(self.resource_limits),
      lifespan: other.lifespan.or(self.lifespan),      
    }
  }

  // Check if policy self commplies to other.
  //
  // "self" is the "offered" (publisher) QoS
  // "other" is the "requested" (subscriber) QoS
  // yes => None (no failure, i.e. are compatible)
  // no => Some(policyId) , where policyId is (any) one of the policies
  // causing incompliance 
  // Compliance (comaptibility) is defined in the table in DDS spec v1.4
  // Section "2.2.3 Supported QoS"
  // This is not symmetric.
  pub fn compliance_failure_wrt(&self, other: &QosPolicies) -> Option<QosPolicyId> {
    trace!("QoS compatibility check - offered: {:?} - requested {:?}", self, other);
    let result = self.compliance_failure_wrt_impl(other);
    trace!("Result: {:?}",result);
    result
  }


  pub fn compliance_failure_wrt_impl (&self, other: &QosPolicies) -> Option<QosPolicyId> {
    // TODO: Check for cases where policy is requested, but not offered (None)

    // check Durability: Offered must be better than or equal to Requested.
    if let (Some(off),Some(req)) = (self.durability, other.durability) {
      if off < req {
        return Some(QosPolicyId::Durability)
      }
    }

    // check Presentation: 
    // * If coherent_access is requsted, it must be offered also. AND
    // * Same for ordered_access. AND
    // * Offered access scope is broader than requested.
    if let (Some(off),Some(req)) = (self.presentation, other.presentation) {
      if   (req.coherent_access && ! off.coherent_access)
        || (req.ordered_access && ! off.ordered_access)
        || (req.access_scope > off.access_scope) {
        return Some(QosPolicyId::Presentation)
      }
    }


    // check Deadline: offered period <= requested period
    if let (Some(off),Some(req)) = (self.deadline, other.deadline) {
      if off.0 > req.0 {
        return Some(QosPolicyId::Deadline)
      }
    }


    // check Latency Budget:
    // offered duration <= requested duration
    if let (Some(off),Some(req)) = (self.latency_budget, other.latency_budget) {
      if off.duration > req.duration {
        return Some(QosPolicyId::LatencyBudget)
      }
    }

    // check Ownership:
    // offered kind == requested kind
    if let (Some(off),Some(req)) = (self.ownership, other.ownership) {
      if off != req {
        return Some(QosPolicyId::Ownership)
      }
    }

    // check Liveliness
    // offered kind >= requested kind
    // Definition: AUTOMATIC < MANUAL_BY_PARTICIPANT < MANUAL_BY_TOPIC
    // AND offered lease_duration <= requested lease_duration
    //
    // See Ord implementation on Liveliness.
    if let (Some(off),Some(req)) = (self.liveliness, other.liveliness) {
      if off < req {
        return Some(QosPolicyId::Liveliness)
      }
    }

    // check Reliability
    // offered kind >= requested kind
    // kind ranking: BEST_EFFORT < RELIABLE
    match (self.reliability, other.reliability) {
      (Some(off),Some(req)) => 
        if off < req { return Some(QosPolicyId::Reliability) },
      (None, Some(policy::Reliability::Reliable{..})) =>
        // request is reliable, but no offer at all. This is bad.
        return Some(QosPolicyId::Reliability),
      _ => (), // otherwise, we shoudl be good to go
    }

    // check Destination Order
    // offered kind >= requested kind
    // kind ranking: BY_RECEPTION_TIMESTAMP < BY_SOURCE_TIMESTAMP
    if let (Some(off),Some(req)) = (self.destination_order, other.destination_order) {
      if off < req {
        return Some(QosPolicyId::DestinationOrder)
      }
    }

    // default value. no incompatibility detected.
    None
  }
}

// put these into a submodule to avoid repeating the word "policy" or "qospolicy"
/// Contains all available QoSPolicies
pub mod policy {
  use crate::structure::{parameter_id::ParameterId, duration::Duration};
  use serde::{Serialize, Deserialize};
  use std::cmp::Ordering;

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

  /// DDS 2.2.3.16 LIFESPAN
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct Lifespan {
    pub duration: Duration,
  }

  /// DDS 2.2.3.4 DURABILITY
  #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
  pub enum Durability {
    Volatile,
    TransientLocal,
    Transient,
    Persistent,
  }

  /// DDS 2.2.3.6 PRESENTATION
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct Presentation {
    pub access_scope: PresentationAccessScope,
    pub coherent_access: bool,
    pub ordered_access: bool,
  }

  /// Access scope that is part of DDS 2.2.3.6 PRESENTATION
  #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
  pub enum PresentationAccessScope {
    Instance,
    Topic,
    Group,
  }

  /// DDS 2.2.3.7 DEADLINE
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
  pub struct Deadline(pub Duration);

  /// DDS 2.2.3.8 LATENCY_BUDGET
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct LatencyBudget {
    pub duration: Duration,
  }

  /// DDS 2.2.3.9 OWNERSHIP
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub enum Ownership {
    Shared,
    Exclusive { strength: i32 }, // This also implements OwnershipStrength
  }

  /// DDS 2.2.3.11 LIVELINESS
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub enum Liveliness {
    Automatic { lease_duration: Duration },
    ManualByParticipant { lease_duration: Duration },
    ManualByTopic { lease_duration: Duration },
  }

  impl Liveliness {
    fn kind_num(&self) -> i32 {
      use Liveliness::*;
      match self {
        Automatic {..} => 0,
        ManualByParticipant {..} => 1,
        ManualByTopic {..} => 2,
      }
    }

    pub fn duration(&self) -> Duration {
      use Liveliness::*;
      match self {
        Automatic {lease_duration} => *lease_duration,
        ManualByParticipant {lease_duration} => *lease_duration,
        ManualByTopic {lease_duration} => *lease_duration,
      }      
    }

  }

  impl Ord for Liveliness {
    fn cmp(&self, other: &Self) -> Ordering {
      // Manual liveliness is greater than automatic, but
      // duration compares in reverse
     other.kind_num().cmp( &other.kind_num())
        .then_with( || self.duration().cmp(&other.duration())
          .reverse() )
    }
  }

  impl PartialOrd for Liveliness {
      fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
          Some(self.cmp(other))
      }
  }


  /// DDS 2.2.3.12 TIME_BASED_FILTER
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct TimeBasedFilter {
    pub minimum_separation: Duration,
  }

  /*
  pub struct Partition {
    pub name: Vec<Vec<u8>>,
  }
  */

  /// DDS 2.2.3.14 RELIABILITY
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub enum Reliability {
    BestEffort,
    Reliable { max_blocking_time: Duration },
  }

  impl Ord for Reliability {
    // max_blocking_time is not compared.
    // TODO: This is kind of bad, because now Eq and Ord are inconsistent:
    // If we have A and B both Reliability::Reliable, but with different
    // max_blocking_time, then Ord will say they are Ordering::Equal,
    // but Eq will say they are not equal.
    fn cmp(&self, other: &Self) -> Ordering {
      use Reliability::*;
      match (self,other) {
        (BestEffort, BestEffort) => Ordering::Equal,
        (BestEffort, Reliable {..}) => Ordering::Less,
        (Reliable{..}, BestEffort) => Ordering::Greater,
        (Reliable{..}, Reliable {..} ) => Ordering::Equal, 
      }
    }
  }

  impl PartialOrd for Reliability {
      fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
          Some(self.cmp(other))
      }
  }

  /// DDS 2.2.3.17 DESTINATION_ORDER
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
  pub enum DestinationOrder {
    ByReceptionTimestamp,
    BySourceTimeStamp,
  }

  /// DDS 2.2.3.18 HISTORY
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub enum History {
    KeepLast { depth: i32 },
    KeepAll,
  }

  /// DDS 2.2.3.19 RESOURCE_LIMITS
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub struct ResourceLimits {
    pub max_samples: i32,
    pub max_instances: i32,
    pub max_samples_per_instance: i32,
  }

  #[derive(Serialize, Deserialize)]
  pub(crate) struct QosData<D>
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

// Utility for parsing RTPS inlineQoS parameters
pub(crate) struct InlineQos {}

impl InlineQos {
  pub fn status_info(
    params: &ParameterList,
    rep_id: RepresentationIdentifier,
  ) -> std::result::Result<StatusInfo, crate::serialization::error::Error> {
    let status_info = params
      .parameters
      .iter()
      .find(|p| p.parameter_id == ParameterId::PID_STATUS_INFO)
      .clone();
    let status_info = match status_info {
      Some(p) => StatusInfo::from_cdr_bytes(&p.value, rep_id)?,
      None => StatusInfo::empty(),
    };

    Ok(status_info)
  }

  pub fn key_hash(
    params: &ParameterList,
    rep_id: RepresentationIdentifier,
  ) -> std::result::Result<KeyHash, crate::serialization::error::Error> {
    let key_hash = params
      .parameters
      .iter()
      .find(|p| p.parameter_id == ParameterId::PID_KEY_HASH)
      .clone();
    let key_hash = match key_hash {
      Some(p) => KeyHash::from_cdr_bytes(&p.value, rep_id)?,
      None => KeyHash::empty(),
    };

    Ok(key_hash)
  }
}


// TODO: helper function to check is a QosPolices object is inconsistent (by itself)

// TODO: helper function to check if two QosPolicies: Reequested and Offered are
// compatible, according to DDS spec 2.2.3
