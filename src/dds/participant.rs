use mio::Token;
use mio_extras::channel as mio_channel;

use std::thread;
use std::collections::HashMap;
use std::time::Duration;

use crate::network::udp_listener::UDPListener;
use crate::network::constant::*;
use crate::dds::dp_event_wrapper::DPEventWrapper;
use crate::dds::reader::*;
use crate::dds::writer::Writer;
use crate::dds::pubsub::*;
use crate::dds::topic::*;
use crate::dds::typedesc::*;
use crate::dds::qos::*;
use crate::dds::values::result::*;
use crate::structure::entity::{Entity, EntityAttributes};
use crate::structure::guid::GUID;
use std::net::Ipv4Addr;

use std::sync::{Arc, Mutex};

pub struct DomainParticipant {
  entity_attributes: EntityAttributes,
  reader_binds: HashMap<Token, mio_channel::Receiver<(Token, Reader)>>,

  subs: Vec<Arc<Mutex<Subscriber>>>,

  // Adding Readers
  sender_add_reader: mio_channel::Sender<Reader>,
  sender_remove_reader: mio_channel::Sender<GUID>,

  // Adding DataReaders
  sender_add_datareader_vec: Vec<mio_channel::Sender<()>>,
  sender_remove_datareader_vec: Vec<mio_channel::Sender<GUID>>,

  // Writers
  add_writer_sender: mio_channel::Sender<Writer>,
  remove_writer_sender: mio_channel::Sender<GUID>,
}

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

    // Adding readers
    let (sender_add_reader, receiver_add_reader) = mio_channel::channel::<Reader>();
    let (sender_remove_reader, receiver_remove_reader) = mio_channel::channel::<GUID>();

    // Writers
    let (add_writer_sender, add_writer_receiver) = mio_channel::channel::<Writer>();
    let (remove_writer_sender, remove_writer_receiver) = mio_channel::channel::<GUID>();

    let new_guid = GUID::new();

    let ev_wrapper = DPEventWrapper::new(
      listeners,
      targets,
      new_guid.guidPrefix,
      TokenReceiverPair {
        token: ADD_READER_TOKEN,
        receiver: receiver_add_reader,
      },
      TokenReceiverPair {
        token: REMOVE_READER_TOKEN,
        receiver: receiver_remove_reader,
      },
      TokenReceiverPair {
        token: ADD_WRITER_TOKEN,
        receiver: add_writer_receiver,
      },
      TokenReceiverPair {
        token: REMOVE_WRITER_TOKEN,
        receiver: remove_writer_receiver,
      },
    );
    thread::spawn(move || DPEventWrapper::event_loop(ev_wrapper));

    // Addind datareaders
    DomainParticipant {
      entity_attributes: EntityAttributes { guid: new_guid },
      reader_binds: HashMap::new(),
      subs: Vec::new(),
      // Adding readers
      sender_add_reader,
      sender_remove_reader,
      // Adding datareaders
      sender_add_datareader_vec: Vec::new(),
      sender_remove_datareader_vec: Vec::new(),
      add_writer_sender,
      remove_writer_sender,
    }
  }

  pub fn add_reader(&self, reader: Reader) {
    self.sender_add_reader.send(reader).unwrap();
  }

  pub fn remove_reader(&self, guid: GUID) {
    let reader_guid = guid; // How to identify reader to be removed?
    self.sender_remove_reader.send(reader_guid).unwrap();
  }

  pub fn add_datareader(&self, _datareader: DataReader, pos: usize) {
    self.sender_add_datareader_vec[pos].send(()).unwrap();
  }

  pub fn remove_datareader(&self, guid: GUID, pos: usize) {
    let datareader_guid = guid; // How to identify reader to be removed?
    self.sender_remove_datareader_vec[pos]
      .send(datareader_guid)
      .unwrap();
  }

  // Publisher and subscriber creation
  //
  // There are no delete function for publisher or subscriber. Deletion is performed by
  // deleting the Publisher or Subscriber object, who upon deletion will notify
  // the DomainParticipant.
  pub fn create_publisher<'a>(&'a self, qos: QosPolicies) -> Result<Publisher<'a>> {
    let add_writer_sender = self.add_writer_sender.clone();

    Ok(Publisher::new(&self, qos.clone(), qos, add_writer_sender))
  }

  pub fn create_subscriber(&mut self, qos: QosPolicies) {
    let (sender_add_datareader, receiver_add_datareader) = mio_channel::channel::<()>();
    let (sender_remove_datareader, receiver_remove_datareader) = mio_channel::channel::<GUID>();

    self.sender_add_datareader_vec.push(sender_add_datareader);
    self
      .sender_remove_datareader_vec
      .push(sender_remove_datareader);

    let mut subscriber = Subscriber::new(
      qos,
      self.sender_add_reader.clone(),
      self.sender_remove_reader.clone(),
      receiver_add_datareader,
      receiver_remove_datareader,
      self.get_guid(),
    );
    //self.subs.get_mut().unwrap().push(subscriber);

    thread::spawn(move || subscriber.subscriber_poll());
  }

  pub fn created_datareader(&self, _qos: QosPolicies) {
    todo!();
  }

  // Topic creation. Data types should be handled as something (potentially) more structured than a String.
  // NOTE: Here we are using &str for topic name. &str is Unicode string, whereas DDS specifes topic name
  // to be a sequence of octets, which would be &[u8] in Rust. This may cause problems if there are topic names
  // with non-ASCII characters. On the other hand, string handling with &str is easier in Rust.
  pub fn create_topic<'a>(
    &'a self,
    name: &str,
    type_desc: TypeDesc,
    qos: QosPolicies,
  ) -> Result<Topic<'a>> {
    let topic = Topic::new(&self, name.to_string(), type_desc, qos);
    Ok(topic)

    // TODO: refine
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

impl Default for DomainParticipant {
  fn default() -> DomainParticipant {
    DomainParticipant::new()
  }
}

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
    let _dp = DomainParticipant::new();

    let sender = UDPSender::new(11401);
    let data: Vec<u8> = vec![0, 1, 2, 3, 4];

    let addrs = vec![SocketAddr::new("127.0.0.1".parse().unwrap(), 7412)];
    sender.send_to_all(&data, &addrs);

    // TODO: get result data from Reader
  }
}
