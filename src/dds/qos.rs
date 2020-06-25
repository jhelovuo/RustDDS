use crate::dds::result::*;


// This is to be implemented by all DomanParticipant, Publisher, Subscriber, DataWriter, DataReader, Topic
pub trait HasQoSPolicy {
  fn get_qos<'a>(self) -> &'a QosPolicies;
  fn set_qos(self, new_qos: &QosPolicies) -> Result<()>;
}

// DDS spec 2.3.3 defines this as "long" with named constants from 0 to 22.
pub enum QosPolicyId {
  //Invalid  // We should represent this using Option<QosPolicyId> where needed
  UserData,  // 1
  Durability,  // 2
  Presentation,  // 3
  Deadline,
  LatencyBudget, // 5
  Ownership,
  OwnershipStrength, // 7
  Liveliness,
  TimeBasedFilter, // 9
  Partition,
  Reliability, // 11
  DestinationOrder,
  History, // 13
  ResourceLimits,
  EntityFactory, // 15
  WriterDataLifeCycle,
  ReaderDataLifeCycle, // 17
  TopicData, // 18
  GroupData,
  TransportPriority, // 20
  Lifespan,
  DurabilityService, // 22
}


#[derive(Clone)]
pub struct QosPolicies {} // placeholders