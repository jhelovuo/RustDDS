use mio::Token;
use mio_extras::channel as mio_channel;

use std::{
  thread,
  thread::JoinHandle,
  collections::HashMap,
  time::Duration,
  sync::{Arc, RwLock},
  ops::Deref,
  net::Ipv4Addr,
};

use crate::network::{udp_listener::UDPListener, constant::*};

use crate::dds::{
  dp_event_wrapper::DPEventWrapper, reader::*, writer::Writer, pubsub::*, topic::*, typedesc::*,
  qos::*, values::result::*,
};

use crate::{
  discovery::{discovery::Discovery, discovery_db::DiscoveryDB},
  structure::{
    entity::{Entity, EntityAttributes},
    guid::GUID,
    dds_cache::DDSCache,
  },
};

#[derive(Clone)]
// This is a smart pointer for DomainPArticipant_Inner for easier manipulation.
pub struct DomainParticipant {
  dpi: Arc<DomainParticipant_Inner>,
  domain_id: u16,
  participant_id: u16,
}

// Send and Sync for actual ability to send between threads
unsafe impl Send for DomainParticipant {}
unsafe impl Sync for DomainParticipant {}

#[allow(clippy::new_without_default)]
impl DomainParticipant {
  pub fn new(domain_id: u16, participant_id: u16) -> DomainParticipant {
    let (reader_update_notification_sender, reader_update_notification_receiver) =
      mio_channel::channel::<()>();
    let (writer_update_notification_sender, writer_update_notification_receiver) =
      mio_channel::channel::<()>();

    let dpi = DomainParticipant_Inner::new(
      domain_id,
      participant_id,
      reader_update_notification_receiver,
      writer_update_notification_receiver,
    );
    let dp = DomainParticipant {
      dpi: Arc::new(dpi),
      domain_id,
      participant_id,
    };

    let discovery = Discovery::new(
      dp.clone(),
      dp.dpi.discovery_db.clone(),
      writer_update_notification_sender,
      reader_update_notification_sender,
    );
    // TODO:
    // FIXME: when do we stop discovery?
    let _discovery_handle = thread::spawn(move || Discovery::discovery_event_loop(discovery));

    dp
  }

  pub fn create_publisher(&self, qos: &QosPolicies) -> Result<Publisher> {
    self.dpi.create_publisher(&self, qos)
  }

  pub fn create_subscriber<'a>(&self, qos: &QosPolicies) -> Result<Subscriber> {
    self.dpi.create_subscriber(&self, qos)
  }

  pub fn create_topic(&self, name: &str, type_desc: TypeDesc, qos: &QosPolicies) -> Result<Topic> {
    self.dpi.create_topic(&self, name, type_desc, qos)
  }

  pub fn domain_id(&self) -> u16 {
    self.domain_id
  }

  pub fn participant_id(&self) -> u16 {
    self.participant_id
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

  // Adding Readers
  sender_add_reader: mio_channel::Sender<Reader>,
  sender_remove_reader: mio_channel::Sender<GUID>,

  // Adding DataReaders
  sender_add_datareader_vec: Vec<mio_channel::Sender<()>>,
  sender_remove_datareader_vec: Vec<mio_channel::Sender<GUID>>,

  // dp_event_wrapper control
  stop_poll_sender: mio_channel::Sender<()>,
  ev_loop_handle: Option<JoinHandle<()>>,

  // Writers
  add_writer_sender: mio_channel::Sender<Writer>,
  remove_writer_sender: mio_channel::Sender<GUID>,

  dds_cache: Arc<RwLock<DDSCache>>,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
}

impl Drop for DomainParticipant_Inner {
  fn drop(&mut self) {
    // if send has an error simply leave as we have lost control of the ev_loop_thread anyways
    match self.stop_poll_sender.send(()) {
      Ok(_) => (),
      _ => return (),
    };

    // handle should always exist
    // ignoring errors on join
    match self.ev_loop_handle.take().unwrap().join() {
      Ok(s) => s,
      _ => (),
    };
  }
}

pub struct SubscriptionBuiltinTopicData {} // placeholder

#[allow(clippy::new_without_default)]
impl DomainParticipant_Inner {
  fn new(
    domain_id: u16,
    participant_id: u16,
    reader_update_notification_receiver: mio_channel::Receiver<()>,
    writer_update_notification_receiver: mio_channel::Receiver<()>,
  ) -> DomainParticipant_Inner {
    // Creating UPD listeners for participantId 0 (change this if necessary)
    let discovery_multicast_listener = UDPListener::new(
      DISCOVERY_SENDER_TOKEN,
      "0.0.0.0",
      get_spdp_well_known_multicast_port(domain_id),
    );
    discovery_multicast_listener
      .join_multicast(&Ipv4Addr::new(239, 255, 0, 1))
      .expect("Unable to join multicast 239.255.0.1:7400");

    let discovery_listener = UDPListener::new(
      DISCOVERY_SENDER_TOKEN,
      "0.0.0.0",
      get_spdp_well_known_unicast_port(domain_id, participant_id),
    );

    let user_traffic_multicast_listener = UDPListener::new(
      USER_TRAFFIC_SENDER_TOKEN,
      "0.0.0.0",
      get_user_traffic_multicast_port(domain_id),
    );
    user_traffic_multicast_listener
      .join_multicast(&Ipv4Addr::new(239, 255, 0, 1))
      .expect("Unable to join multicast 239.255.0.1:7401");

    let user_traffic_listener = UDPListener::new(
      USER_TRAFFIC_SENDER_TOKEN,
      "0.0.0.0",
      get_user_traffic_unicast_port(domain_id, participant_id),
    );

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

    let a_r_cache = Arc::new(RwLock::new(DDSCache::new()));

    let discovery_db = Arc::new(RwLock::new(DiscoveryDB::new()));

    let (stop_poll_sender, stop_poll_receiver) = mio_channel::channel::<()>();

    let ev_wrapper = DPEventWrapper::new(
      new_guid,
      listeners,
      a_r_cache.clone(),
      discovery_db.clone(),
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
      stop_poll_receiver,
      reader_update_notification_receiver,
      writer_update_notification_receiver,
    );
    // Launch the background thread for DomainParticipant
    let ev_loop_handle = thread::spawn(move || ev_wrapper.event_loop());

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
      stop_poll_sender,
      ev_loop_handle: Some(ev_loop_handle),
      add_writer_sender,
      remove_writer_sender,
      dds_cache: Arc::new(RwLock::new(DDSCache::new())),
      discovery_db: discovery_db,
    }
  }

  pub fn get_dds_cache(&self) -> Arc<RwLock<DDSCache>> {
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
    &self,
    domain_participant: &DomainParticipant,
    qos: &QosPolicies,
  ) -> Result<Publisher> {
    let add_writer_sender = domain_participant.get_add_writer_sender().clone();

    Ok(Publisher::new(
      domain_participant.clone(),
      qos.clone(),
      qos.clone(),
      add_writer_sender,
    ))
  }

  pub fn create_subscriber(
    &self,
    domain_participant: &DomainParticipant,
    qos: &QosPolicies,
  ) -> Result<Subscriber> {
    Ok(Subscriber::new(
      domain_participant.clone(),
      qos.clone(),
      self.sender_add_reader.clone(),
      self.sender_remove_reader.clone(),
    ))
  }

  // Topic creation. Data types should be handled as something (potentially) more structured than a String.
  // NOTE: Here we are using &str for topic name. &str is Unicode string, whereas DDS specifes topic name
  // to be a sequence of octets, which would be &[u8] in Rust. This may cause problems if there are topic names
  // with non-ASCII characters. On the other hand, string handling with &str is easier in Rust.
  pub fn create_topic(
    &self,
    domain_participant: &DomainParticipant,
    name: &str,
    type_desc: TypeDesc,
    qos: &QosPolicies,
  ) -> Result<Topic> {
    let topic = Topic::new(domain_participant, name.to_string(), type_desc, &qos);
    Ok(topic)

    // TODO: refine
  }

  // Do not implement contentfilteredtopics or multitopics (yet)

  pub fn find_topic(self, _name: &str, _timeout: Duration) -> Result<Topic> {
    unimplemented!()
  }

  // get_builtin_subscriber (why would we need this?)

  // ignore_* operations. TODO: Do we needa any of those?

  // delete_contained_entities is not needed. Data structures shoud be designed so that lifetime of all
  // created objects is within the lifetime of DomainParticipant. Then such deletion is implicit.

  pub fn assert_liveliness(self) {
    unimplemented!()
  }

  // The following methods are not for application use.

  pub(crate) fn get_add_reader_sender(&self) -> mio_channel::Sender<Reader> {
    self.sender_add_reader.clone()
  }

  pub(crate) fn get_remove_reader_sender(&self) -> mio_channel::Sender<GUID> {
    self.sender_remove_reader.clone()
  }

  pub(crate) fn get_add_writer_sender(&self) -> mio_channel::Sender<Writer> {
    self.add_writer_sender.clone()
  }

  pub(crate) fn get_remove_writer_sender(&self) -> mio_channel::Sender<GUID> {
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
  use mio_extras::channel as mio_channel;
  use std::{thread, net::SocketAddr};
  use crate::speedy::Writable;
  use crate::{
    dds::{qos::QosPolicies, typedesc::TypeDesc, writer::Writer},
    network::{udp_sender::UDPSender, constant::get_user_traffic_unicast_port},
    test::random_data::RandomData,
    structure::{
      locator::{LocatorKind, Locator},
      guid::{EntityId, GUID},
      sequence_number::{SequenceNumber, SequenceNumberSet},
    },
    submessages::{EntitySubmessage, AckNack, SubmessageFlag, SubmessageHeader, SubmessageKind},
    common::bit_set::BitSetRef,
    serialization::{SubMessage, Message},
    messages::{
      protocol_version::ProtocolVersion, header::Header, vendor_id::VendorId,
      protocol_id::ProtocolId,
    },
  };
  use super::{DomainParticipant_Inner, DomainParticipant};
  use speedy::Endianness;
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
  #[test]
  fn dp_writer_hearbeat_test() {
    let domain_participant = DomainParticipant::new(5, 0);
    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();
    thread::sleep(time::Duration::milliseconds(1000).to_std().unwrap());
    let publisher = domain_participant
      .create_publisher(&qos.clone())
      .expect("Failed to create publisher");
    thread::sleep(time::Duration::milliseconds(1000).to_std().unwrap());
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos.clone())
      .expect("Failed to create topic");
    thread::sleep(time::Duration::milliseconds(1000).to_std().unwrap());
    let mut _data_writer = publisher
      .create_datawriter::<RandomData>(None, &topic, &qos.clone())
      .expect("Failed to create datawriter");

    thread::sleep(time::Duration::seconds(5).to_std().unwrap());
  }

  #[test]
  fn dp_recieve_acknack_message_test() {
    // TODO SEND ACKNACK
    let domain_participant = DomainParticipant::new(6, 0);

    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    let publisher = domain_participant
      .create_publisher(&qos.clone())
      .expect("Failed to create publisher");
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos.clone())
      .expect("Failed to create topic");
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    let mut _data_writer = publisher
      .create_datawriter::<RandomData>(None, &topic, &qos.clone())
      .expect("Failed to create datawriter");

    let portNumber: u16 = get_user_traffic_unicast_port(5, 0);
    let _sender = UDPSender::new(1234);
    let mut m: Message = Message::new();

    let a: AckNack = AckNack {
      reader_id: EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
      writer_id: EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
      reader_sn_state: SequenceNumberSet {
        base: SequenceNumber::default(),
        set: BitSetRef::new(),
      },
      count: 1,
    };
    let subHeader: SubmessageHeader = SubmessageHeader {
      submessage_id: SubmessageKind::ACKNACK,
      flags: SubmessageFlag {
        flags: 0b0000000_u8,
      },
      submessage_length: 24,
    };

    let s: SubMessage = SubMessage {
      header: subHeader,
      intepreterSubmessage: None,
      submessage: Some(EntitySubmessage::AckNack(
        a,
        SubmessageFlag {
          flags: 0b0000000_u8,
        },
      )),
    };
    let h = Header {
      protocol_id: ProtocolId::default(),
      protocol_version: ProtocolVersion { major: 2, minor: 3 },
      vendor_id: VendorId::VENDOR_UNKNOWN,
      guid_prefix: GUID::new().guidPrefix,
    };
    m.set_header(h);
    m.add_submessage(SubMessage::from(s));
    let _data: Vec<u8> = m.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap();
    println!("data to send via udp: {:?}", _data);
    let loca = Locator {
      kind: LocatorKind::LOCATOR_KIND_UDPv4,
      port: portNumber as u32,
      address: [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
      ],
      //address: [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0x00, 0x00, 0x01],
    };
    let locas = vec![loca];
    _sender.send_to_locator_list(&_data, &locas);

    thread::sleep(time::Duration::seconds(5).to_std().unwrap());
  }
}
