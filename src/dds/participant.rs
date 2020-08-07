use mio::Token;
use mio_extras::channel as mio_channel;

use std::{thread, collections::HashMap, time::Duration, sync::{Arc, RwLock}, rc::Rc};

use crate::network::constant::*;

use crate::dds::{
  dp_event_wrapper::DPEventWrapper, reader::*, writer::Writer, pubsub::*, topic::*, typedesc::*,
  qos::*, values::result::*,
};

use crate::structure::{
  entity::{Entity, EntityAttributes},
  guid::GUID,
  dds_cache::DDSHistoryCache,
};

#[derive(Clone)]
// This is a smart pointer for DomainPArticipant_Inner for easier manipulation.
pub struct DomainParticipant {
  dpi: Arc<DomainParticipant_Inner>,
}

impl DomainParticipant {
  pub fn new() -> DomainParticipant {
    let dpi = DomainParticipant_Inner::new();
    DomainParticipant {dpi: Arc::new(dpi) }
  }

  pub fn create_publisher<'a>(&'a self, qos: QosPolicies) -> Result<Publisher> {
    self.dpi.create_publisher(&self,qos) // pass in Arc to DomainParticipant
  }

  pub fn create_subscriber(&self, qos: QosPolicies) -> Subscriber {
    self.dpi.create_subscriber(&self,qos)
  }

  pub fn create_topic<'a>(&'a self, name: &str,type_desc: TypeDesc, qos: QosPolicies) 
      -> Result<Topic> {
    self.dpi.create_topic(&self,name,type_desc,qos)
  }

}

impl Deref for DomainParticipant {
  type Target = DomainParticipant_Inner;
  fn deref(&self) -> &DomainParticipant_Inner {
    &self.dpi
  }
}


// This is the actual working DomainParticipant.
pub struct DomainParticipant_Inner {
  entity_attributes: EntityAttributes,
  reader_binds: HashMap<Token, mio_channel::Receiver<(Token, Reader)>>,
  //ddscache: Arc<RwLock<DDSCache>>,

  // Adding Readers
  sender_add_reader: mio_channel::Sender<Reader>,
  sender_remove_reader: mio_channel::Sender<GUID>,

  // Adding DataReaders
  sender_add_datareader_vec: Vec<mio_channel::Sender<()>>,
  sender_remove_datareader_vec: Vec<mio_channel::Sender<GUID>>,

  // Writers
  add_writer_sender: mio_channel::Sender<Writer>,
  remove_writer_sender: mio_channel::Sender<GUID>,

  dds_cache : Arc<RwLock<DDSCache>>,
}

pub struct SubscriptionBuiltinTopicData {} // placeholder

impl DomainParticipant_Inner {
  // TODO: there might be a need to set participant id (thus calculating ports accordingly)
  pub fn new() -> DomainParticipant_Inner {
    // let discovery_multicast_listener =
    //   UDPListener::new(DISCOVERY_MUL_LISTENER_TOKEN, "0.0.0.0", 7400);
    // discovery_multicast_listener
    //   .join_multicast(&Ipv4Addr::new(239, 255, 0, 1))
    //   .expect("Unable to join multicast 239.255.0.1:7400");

    // let discovery_listener = UDPListener::new(DISCOVERY_LISTENER_TOKEN, "0.0.0.0", 7412);

    // let user_traffic_multicast_listener =
    //   UDPListener::new(USER_TRAFFIC_MUL_LISTENER_TOKEN, "0.0.0.0", 7401);
    // user_traffic_multicast_listener
    //   .join_multicast(&Ipv4Addr::new(239, 255, 0, 1))
    //   .expect("Unable to join multicast 239.255.0.1:7401");

    // let user_traffic_listener = UDPListener::new(USER_TRAFFIC_LISTENER_TOKEN, "0.0.0.0", 7413);

    let listeners = HashMap::new();
    // listeners.insert(DISCOVERY_MUL_LISTENER_TOKEN, discovery_multicast_listener);
    // listeners.insert(DISCOVERY_LISTENER_TOKEN, discovery_listener);
    // listeners.insert(
    //   USER_TRAFFIC_MUL_LISTENER_TOKEN,
    //   user_traffic_multicast_listener,
    // );
    // listeners.insert(USER_TRAFFIC_LISTENER_TOKEN, user_traffic_listener);

    let targets = HashMap::new();

    // Adding readers
    let (sender_add_reader, receiver_add_reader) = mio_channel::channel::<Reader>();
    let (sender_remove_reader, receiver_remove_reader) = mio_channel::channel::<GUID>();

    // Writers
    let (add_writer_sender, add_writer_receiver) = mio_channel::channel::<Writer>();
    let (remove_writer_sender, remove_writer_receiver) = mio_channel::channel::<GUID>();

    let new_guid = GUID::new();

    let a_r_cache = Arc::new(RwLock::new(DDSCache::new()));

    let ev_wrapper = DPEventWrapper::new(
      listeners,
      a_r_cache.clone(),
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
    // Launch the background thread for DomainParticipant
    thread::spawn(move || ev_wrapper.event_loop() );

    DomainParticipant_Inner {
      entity_attributes: EntityAttributes { guid: new_guid },
      reader_binds: HashMap::new(),
      //ddscache: a_r_cache,
      // Adding readers
      sender_add_reader,
      sender_remove_reader,
      // Adding datareaders
      sender_add_datareader_vec: Vec::new(),
      sender_remove_datareader_vec: Vec::new(),
      add_writer_sender,
      remove_writer_sender,
      dds_cache : Arc::new(RwLock::new(DDSCache::new())),
    }
  }

  pub fn get_dds_cache(&self) -> Arc<RwLock<DDSCache>>{
    return self.dds_cache.clone();
  }

  pub fn add_reader(&self, reader: Reader) {
    self.sender_add_reader.send(reader).unwrap();
  }

  pub fn remove_reader(&self, guid: GUID) {
    let reader_guid = guid; // How to identify reader to be removed?
    self.sender_remove_reader.send(reader_guid).unwrap();
  }

  /* removed due to architecture change.
  pub fn add_datareader(&self, _datareader: DataReader, pos: usize) {
    self.sender_add_datareader_vec[pos].send(()).unwrap();
  }

  pub fn remove_datareader(&self, guid: GUID, pos: usize) {
    let datareader_guid = guid; // How to identify reader to be removed?
    self.sender_remove_datareader_vec[pos]
      .send(datareader_guid)
      .unwrap();
  }
  */

  // Publisher and subscriber creation
  //
  // There are no delete function for publisher or subscriber. Deletion is performed by
  // deleting the Publisher or Subscriber object, who upon deletion will notify
  // the DomainParticipant.
  pub fn create_publisher(
    domain_participant: Arc<DomainParticipant>,
    qos: QosPolicies,
  ) -> Result<Rc<Publisher>> {
    let add_writer_sender = domain_participant.get_add_writer_sender().clone();

    Ok(Rc::new(Publisher::new(
      domain_participant,
      qos.clone(),
      qos,
      add_writer_sender,
    )))
  }

  pub fn create_subscriber(
    domain_participant: Arc<DomainParticipant>,
    qos: QosPolicies,
  ) -> Result<Rc<Subscriber>> {
    let subscriber = Subscriber::new(domain_participant, qos);

    Ok(Rc::new(subscriber))
  }

  // Topic creation. Data types should be handled as something (potentially) more structured than a String.
  // NOTE: Here we are using &str for topic name. &str is Unicode string, whereas DDS specifes topic name
  // to be a sequence of octets, which would be &[u8] in Rust. This may cause problems if there are topic names
  // with non-ASCII characters. On the other hand, string handling with &str is easier in Rust.
  pub fn create_topic(
    domain_participant: Arc<DomainParticipant>,
    name: &str,
    type_desc: TypeDesc,
    qos: QosPolicies,
  ) -> Result<Rc<Topic>> {
    let topic = Topic::new(domain_participant, name.to_string(), type_desc, qos);
    Ok(Rc::new(topic))

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

  pub fn get_add_reader_sender(&self) -> mio_channel::Sender<Reader> {
    self.sender_add_reader.clone()
  }

  pub fn get_remove_reader_sender(&self) -> mio_channel::Sender<GUID> {
    self.sender_remove_reader.clone()
  }

  pub fn get_add_writer_sender(&self) -> mio_channel::Sender<Writer> {
    self.add_writer_sender.clone()
  }

  pub fn get_remove_writer_sender(&self) -> mio_channel::Sender<GUID> {
    self.remove_writer_sender.clone()
  }
} // impl

impl Entity for DomainParticipant_Inner {
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
  }
}

impl std::fmt::Debug for DomainParticipant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DomainParticipant")
      .field("Guid", &self.get_guid())
      .finish()
  }
}

#[cfg(test)]
mod tests {
  // use super::*;

  use std::net::SocketAddr;
  use crate::network::udp_sender::UDPSender;
  // TODO: improve basic test when more or the structure is known
  #[test]
  fn dp_basic_domain_participant() {
    // let _dp = DomainParticipant::new();

    let sender = UDPSender::new(11401);
    let data: Vec<u8> = vec![0, 1, 2, 3, 4];

    let addrs = vec![SocketAddr::new("127.0.0.1".parse().unwrap(), 7412)];
    sender.send_to_all(&data, &addrs);

    // TODO: get result data from Reader
  }
}
