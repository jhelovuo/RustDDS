use mio::Token;
use mio_extras::channel as mio_channel;

use std::thread;
use std::collections::HashMap;

use crate::network::udp_listener::UDPListener;
use crate::dds::dp_event_wrapper::DPEventWrapper;
use crate::structure::result::*;
use crate::structure::duration::*;

pub struct DomainParticipant {
  senders: HashMap<Token, mio_channel::Sender<UDPListener>>,
}


struct Publisher {} // placeholders
struct Subscriber {} 
struct Topic {}
struct QosPolicies {}
struct TypeDesc {}

impl DomainParticipant {
  pub fn new() -> DomainParticipant {
    let mut senders = HashMap::new();
    let ev_wrapper = DPEventWrapper::new(&mut senders);
    thread::spawn(move || DPEventWrapper::event_loop(ev_wrapper));
    DomainParticipant { senders: senders }
   }
  


  // Publisher and subscriber creation
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
