use mio::Token;
use mio_extras::channel as mio_channel;

use std::thread;
use std::collections::HashMap;
use std::time::Duration;

use crate::network::udp_listener::UDPListener;
use crate::network::udp_sender::UDPSender;
use crate::network::constant::*;
use crate::dds::dp_event_wrapper::DPEventWrapper;
use crate::structure::result::*;

pub struct DomainParticipant {
  readers: HashMap<Token, mio_channel::Sender<UDPListener>>,
  writers: HashMap<Token, mio_channel::Sender<UDPSender>>,
}

struct Publisher {} // placeholders
struct Subscriber {}
struct Topic {}
struct QosPolicies {}
struct TypeDesc {}

impl DomainParticipant {
  pub fn new() -> DomainParticipant {
    let mut readers = HashMap::new();
    let mut writers = HashMap::new();
    let ev_wrapper = DPEventWrapper::new(&mut readers, &mut writers);
    thread::spawn(move || DPEventWrapper::event_loop(ev_wrapper));
    DomainParticipant {
      readers: readers,
      writers: writers,
    }
  }

  // Publisher and subscriber creation
  //
  // There are no delete function for publisher or subscriber. Deletion is performed by
  // deleting the Publisher or Subscriber object, who upon deletion will notify
  // the DomainParticipant.
  pub fn create_publisher(self, qos: QosPolicies) -> Result<Publisher> {
    unimplemented!()
  }

  pub fn create_subsrciber(self, qos: QosPolicies) -> Result<Subscriber> {
    unimplemented!()
  }

  // Topic creation. Data types should be handled as something (potentially) more structured than a String.
  pub fn create_topic(self, name: &str, type_desc: TypeDesc, qos: QosPolicies) -> Result<Topic> {
    unimplemented!()
  }

  // Do not implement contentfilteredtopics or multitopics (yet)

  pub fn find_topic(self, name: &str, timeout: Duration) -> Result<Topic> {
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

  pub fn add_reader(&self, listener: UDPListener) {
    let readers = self.readers.get(&ADD_UDP_LISTENER_TOKEN);
    match readers {
      Some(reader) => reader.send(listener).expect("Failed to add listener."),
      None => return,
    };
  }

  pub fn add_writer(&self, sender: UDPSender) {
    let writers = self.writers.get(&ADD_UDP_SENDER_TOKEN);
    match writers {
      Some(writer) => writer.send(sender).expect("Failed to add writer."),
      None => return,
    };
  }
} // impl

#[cfg(test)]
mod tests {
  use super::*;

  // #[test]
  // fn dp_single_address_listener() {
  //   let participant = DomainParticipant::new();

  //   let mut listener = UDPListener::new("127.0.0.1", 10001);
  //   listener.add_listen_address("127.0.0.1", 11001);
  //   participant.add_udp_listener(listener);

  //   let mut sender = UDPSender::new(11001);
  //   let data: Vec<u8> = vec![0, 1, 2, 3, 4];
  //   sender.add_send_address("127.0.0.1", 10001);
  //   sender.send_to_all(&data);

  //   let rec_data = participant.get_last_messages();

  //   assert_eq!(rec_data.len(), 1);
  //   assert_eq!(rec_data[0].len(), 5);
  //   assert_eq!(rec_data[0], data);
  // }
}
