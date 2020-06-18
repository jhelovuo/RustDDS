use mio::{Poll, Events, Token, Ready,PollOpt};
use std::thread;
use std::collections::HashMap;

use crate::network::udp_listener::UDPListener;
use crate::network::constant::STOP_POLL_TOKEN;
use crate::structure::result::*;
use crate::structure::duration::*;

struct DomainParticipant {
  poll: Poll,
  events: Events,
  listeners: HashMap<Token, UDPListener>,
}


struct Publisher {} // placeholders
struct Subscriber {} 
struct Topic {}
struct QosPolicies {}
struct TypeDesc {}

impl DomainParticipant {
  pub fn new() -> DomainParticipant {
    DomainParticipant {
      poll: Poll::new().expect("Unable to create new poll."),
      events: Events::with_capacity(1024),
      listeners: HashMap::new(),
    }
  }

  pub fn register_udp_listener(&mut self, mut listener: UDPListener) {
    let t = *listener.token();
    self.poll
      //.registry()
      .register(listener.mio_socket(), t, Ready::readable(),PollOpt::edge());
    self.listeners.insert(*listener.token(), listener);
  }

  pub fn event_loop(&mut self) {
    loop {
      self
        .poll
        .poll(&mut self.events, None)
        .expect("Failed in waiting of poll.");

      for event in &self.events {
        if event.token() == STOP_POLL_TOKEN {
          return;
        }

        let listener = self.listeners.get(&event.token());
        let mut datas: Vec<Vec<u8>> = vec![];
        match listener {
          Some(l) => {
            while let data = l.get_message() {
              if data.is_empty() {
                break;
              }
              datas.push(data);
            }
          }
          None => continue,
        }
      }
    }
  } // fn

  
  // Published and subscriber creation
  //
  // There are no delete function for publisher or subscriber. Deletion is performed by
  // deleting the Publisher or Subscriber object, who upon deletion will notify
  // the DomainParticipant.
  pub fn create_publisher(self,  qos: QosPolicies ) -> Result<Publisher> {
    unimplemented!()
  }

  pub fn create_subsrciber(self, qos: QosPolicies ) -> Result<Subscriber> {
    unimplemented!()
  }

  // Topic creation. Data types should be handled as something (potentially) more structured than a String.
  pub fn create_topic(self, name: &str, type_desc: TypeDesc, qos: QosPolicies) -> Result<Topic> {
    unimplemented!()
  }  

  // Do not implement contentfilteredtopics or multitopics (yet)

  pub fn find_topic(self, name: &str, timeout: Duration_t) -> Result<Topic> {
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


}  // impl
