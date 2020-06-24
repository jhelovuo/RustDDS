use mio::Token;
use mio_extras::channel as mio_channel;

use std::thread;
use std::collections::HashMap;
use std::time::Duration;

use crate::network::udp_listener::UDPListener;
use crate::network::constant::*;
use crate::dds::dp_event_wrapper::DPEventWrapper;
use crate::dds::reader::Reader;
use crate::dds::pubsub::*;
use crate::dds::topic::*;
use crate::structure::result::*;
use crate::structure::entity::{Entity, EntityAttributes};
use crate::structure::guid::GUID;
use std::net::Ipv4Addr;

pub struct DomainParticipant {
  entity_attributes: EntityAttributes,
  reader_binds: HashMap<Token, mio_channel::Receiver<(Token, Reader)>>,
}

// This is to be implemented by all DomanParticipant, Publisher, Subscriber, DataWriter, DataReader, Topic
pub trait HasQoSPolicy {
  fn get_qos<'a>(self) -> &'a QosPolicies;
  fn set_qos(self, new_qos: &QosPolicies) -> Result<()>;
}

#[derive(Clone)]
pub struct QosPolicies {} // placeholders

pub struct TypeDesc {} // placeholders
pub struct SubscriptionBuiltinTopicData {} // placeholder

impl DomainParticipant {
  // TODO: there might be a need to set participant id (thus calculating ports accordingly)
  pub fn new() -> DomainParticipant {
    let discovery_multicast_listener =
      UDPListener::new(DISCOVERY_MUL_LISTENER_TOKEN, "0.0.0.0", 7400);
    discovery_multicast_listener
      .join_multicast(&Ipv4Addr::new(239, 255, 0, 1))
      .expect("Unable to join multicast 239.255.0.1:7400");

    let discovery_listener = UDPListener::new(DISCOVERY_LISTENER_TOKEN, "0.0.0.0", 7412);

    let user_traffic_multicast_listener =
      UDPListener::new(USER_TRAFFIC_MUL_LISTENER_TOKEN, "0.0.0.0", 7401);
    user_traffic_multicast_listener
      .join_multicast(&Ipv4Addr::new(239, 255, 0, 1))
      .expect("Unable to join multicast 239.255.0.1:7401");

    let user_traffic_listener = UDPListener::new(USER_TRAFFIC_LISTENER_TOKEN, "0.0.0.0", 7413);

    let mut listeners = HashMap::new();
    listeners.insert(DISCOVERY_MUL_LISTENER_TOKEN, discovery_multicast_listener);
    listeners.insert(DISCOVERY_LISTENER_TOKEN, discovery_listener);
    listeners.insert(
      USER_TRAFFIC_MUL_LISTENER_TOKEN,
      user_traffic_multicast_listener,
    );
    listeners.insert(USER_TRAFFIC_LISTENER_TOKEN, user_traffic_listener);

    let targets = HashMap::new();

    let ev_wrapper = DPEventWrapper::new(listeners, targets);
    thread::spawn(move || DPEventWrapper::event_loop(ev_wrapper));
    DomainParticipant {
      entity_attributes: EntityAttributes { guid: GUID::new() },
      reader_binds: HashMap::new(),
    }
  }

  // Publisher and subscriber creation
  //
  // There are no delete function for publisher or subscriber. Deletion is performed by
  // deleting the Publisher or Subscriber object, who upon deletion will notify
  // the DomainParticipant.
  pub fn create_publisher<'a>(&'a self, _qos: QosPolicies) -> Result<Publisher<'a>> {
    unimplemented!()
  }

  pub fn create_subsrciber<'a>(&'a self, _qos: QosPolicies) -> Result<Subscriber<'a>> {
    unimplemented!()
  }

  // Topic creation. Data types should be handled as something (potentially) more structured than a String.
  pub fn create_topic<'a>(
    &'a self,
    _name: &str,
    _type_desc: TypeDesc,
    _qos: QosPolicies,
  ) -> Result<Topic<'a>> {
    unimplemented!()
  }

  // Do not implement contentfilteredtopics or multitopics (yet)

  pub fn find_topic(self, _name: &str, _timeout: Duration) -> Result<Topic> {
    unimplemented!()
  }

  // TopicDescription? shoudl that be implemented? is is necessary?

  // get_builtin_subscriber (why would we need this?)

  // ignore_* operations. TODO: Do we needa any of those?

  // delete_contained_entities is not needed. Data structures shoud be designed so that lifetime of all
  // created objects is within the lifetime of DomainParticipant. Then such deletion is implicit.

  pub fn assert_liveliness(self) {
    unimplemented!()
  }
} // impl

impl Entity for DomainParticipant {
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::net::SocketAddr;
  use crate::network::udp_sender::UDPSender;
  // TODO: improve basic test when more or the structure is known
  #[test]
  fn dp_basic_domain_participant() {
    let dp = DomainParticipant::new();

    let sender = UDPSender::new(11401);
    let data: Vec<u8> = vec![0, 1, 2, 3, 4];

    let addrs = vec![SocketAddr::new("127.0.0.1".parse().unwrap(), 7412)];
    sender.send_to_all(&data, &addrs);

    // TODO: get result data from Reader
  }
}
