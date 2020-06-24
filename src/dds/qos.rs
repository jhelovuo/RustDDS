use crate::dds::result::*;


// This is to be implemented by all DomanParticipant, Publisher, Subscriber, DataWriter, DataReader, Topic
pub trait HasQoSPolicy {
  fn get_qos<'a>(self) -> &'a QosPolicies;
  fn set_qos(self, new_qos: &QosPolicies) -> Result<()>;
}


#[derive(Clone)]
pub struct QosPolicies {} // placeholders