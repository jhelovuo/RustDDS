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
use crate::structure::result::*;


pub struct DomainParticipant {
  add_udp_sender_channel: mio_channel::Sender<(Token, UDPListener)>,
  reader_binds: HashMap<Token, mio_channel::Receiver<(Token, Reader)>>,
}

// This is to be implemented by all DomanParticipant, Publisher, Subscriber, DataWriter, DataReader, Topic
pub trait HasQoSPolicy {
  fn get_qos<'a>(self) -> &'a QosPolicies;
  fn set_qos(self, new_qos: &QosPolicies) -> Result<()>;
}


pub struct Topic {} // placeholders: move these to separate modules as needed

#[derive(Clone)]
pub struct QosPolicies {} // placeholders

pub struct TypeDesc {} // placeholders
pub struct DataReader {} // placeholders
pub struct DataWriter {} // placeholders


impl DomainParticipant {
  pub fn new() -> DomainParticipant {
    // TODO: add mandatory DDS upd listeners and accompanying targets

    // TODO: send add_listener_channel_sender to all places it's needed
    let (add_listener_channel_sender, add_listener_channel_receiver) = mio_channel::channel();

    let listeners = HashMap::new();
    let targets = HashMap::new();
    let ev_wrapper = DPEventWrapper::new(add_listener_channel_receiver, listeners, targets);
    thread::spawn(move || DPEventWrapper::event_loop(ev_wrapper));
    DomainParticipant {
      add_udp_sender_channel: add_listener_channel_sender,
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
  pub fn create_topic(self, _name: &str, _type_desc: TypeDesc, _qos: QosPolicies) -> Result<Topic> {
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

  pub fn add_listener(&self, token: Token, listener: UDPListener) {
    let ss = self.add_udp_sender_channel.send((token, listener));
    match ss {
      Ok(_) => return,
      Err(e) => println!("{}", e.to_string()),
    }
  }
} // impl



#[cfg(test)]
mod tests {
  use super::*;

  use std::net::SocketAddr;
  use crate::network::udp_sender::UDPSender;

  // TODO: improve basic test when more or the structure is known
  #[test]
  fn dp_basic_domain_participant() {
    let dp = DomainParticipant::new();

    let token = START_FREE_TOKENS;
    let listener = UDPListener::new(token.clone(), "127.0.0.1", 10401);
    dp.add_listener(token, listener);

    let mut sender = UDPSender::new(11401);
    let data: Vec<u8> = vec![0, 1, 2, 3, 4];

    let addrs = vec![SocketAddr::new("127.0.0.1".parse().unwrap(), 10401)];
    sender.send_to_all(&data, &addrs);

    // TODO: get result data from Reader
  }
}
