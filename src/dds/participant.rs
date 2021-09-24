//use mio::Token;
use mio_extras::channel as mio_channel;
#[allow(unused_imports)]
use log::{error, debug, info, warn, trace};

use std::{
  thread,
  thread::JoinHandle,
  collections::HashMap,
  time::Duration,
  sync::{Arc, RwLock, Mutex, Weak},
  net::Ipv4Addr,
};

use crate::log_and_err_internal;
use crate::{
  discovery::data_types::topic_data::DiscoveredTopicData,
  discovery::discovery::DiscoveryCommand,
  network::{udp_listener::UDPListener, constant::*},
};

use crate::dds::{
  dp_event_loop::DPEventLoop, reader::*, writer::Writer, pubsub::*, topic::*, typedesc::*, qos::*,
  values::result::*,
};

use crate::{
  discovery::{discovery::Discovery, discovery_db::DiscoveryDB},
  structure::{
    entity::{RTPSEntity},
    guid::GUID,
    dds_cache::DDSCache,
  },
};

use super::dp_event_loop::DomainInfo;

/// DDS DomainParticipant generally only one per domain per machine should be
/// active
#[derive(Clone)]
// This is a smart pointer for DomainParticipant_Inner for easier manipulation.
pub struct DomainParticipant {
  dpi: Arc<Mutex<DomainParticipant_Disc>>,
}

#[allow(clippy::new_without_default)]
impl DomainParticipant {
  /// # Examples
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// ```
  pub fn new(domain_id: u16) -> Result<DomainParticipant> {
    trace!("DomainParticipant construct start");
    let (djh_sender, djh_receiver) = mio_channel::channel();
    let mut dpd = DomainParticipant_Disc::new(domain_id, djh_receiver)?;

    let discovery_updated_sender = match dpd.discovery_updated_sender.take() {
      Some(dus) => dus,
      // this error should never happen
      None => return log_and_err_internal!("Unable to receive Discovery Updated Sender."),
    };

    let discovery_command_receiver = match dpd.discovery_command_receiver.take() {
      Some(dsr) => dsr,
      // this error should never happen
      None => return log_and_err_internal!("Unable to get Discovery Command Receiver."),
    };

    let dp = DomainParticipant {
      dpi: Arc::new(Mutex::new(dpd)),
    };

    let (discovery_started_sender, discovery_started_receiver) =
      std::sync::mpsc::channel::<Result<()>>();

    // Construct and start background thread
    let dp_clone = dp.weak_clone();
    let disc_db_clone = dp.discovery_db();
    let discovery_handle = thread::Builder::new()
      .name("RustDDS discovery thread".to_string())
      .spawn(move || { 
          let discovery = Discovery::new(
            dp_clone,
            disc_db_clone,
            discovery_started_sender,
            discovery_updated_sender,
            discovery_command_receiver,
          );
          Discovery::discovery_event_loop(discovery)
        })?;
    djh_sender.send(discovery_handle).unwrap_or(());

    debug!("Waiting for discovery to start"); // blocking until discovery answers
    match discovery_started_receiver.recv_timeout(Duration::from_secs(10)) {
      Ok(Ok(())) => {
        // normal case
        info!("Discovery started. Participant constructed.");
        Ok(dp)
      }
      Ok(Err(e)) => {
        std::mem::drop(dp);
        log_and_err_internal!("Failed to start discovery thread: {:?}", e)
      }
      Err(e) => log_and_err_internal!("Discovery thread channel error: {:?}", e),
    }
  }

  /// Creates DDS Publisher
  ///
  /// # Arguments
  ///
  /// * `qos` - Takes [qos policies](qos/struct.QosPolicies.html) for publisher
  ///   and given to DataWriter as default.
  ///
  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos);
  /// ```
  pub fn create_publisher(&self, qos: &QosPolicies) -> Result<Publisher> {
    let w = self.weak_clone(); // this must be done first to avoid deadlock
    self.dpi.lock().unwrap().create_publisher(&w, qos)
  }

  /// Creates DDS Subscriber
  ///
  /// # Arguments
  ///
  /// * `qos` - Takes [qos policies](qos/struct.QosPolicies.html) for subscriber
  ///   and given to DataReader as default.
  ///
  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let subscriber = domain_participant.create_subscriber(&qos);
  /// ```
  pub fn create_subscriber(&self, qos: &QosPolicies) -> Result<Subscriber> {
    // println!("DP(outer): create_subscriber");
    let w = self.weak_clone(); // do this first, avoid deadlock
    self.dpi.lock().unwrap().create_subscriber(&w, qos)
  }

  /// Create DDS Topic
  ///
  /// # Arguments
  ///
  /// * `name` - Name of the topic.
  /// * `type_desc` - Name of the type this topic is supposed to deliver.
  /// * `qos` - Takes [qos policies](qos/struct.QosPolicies.html) that are
  ///   distributed to DataReaders and DataWriters.
  ///
  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// use rustdds::dds::data_types::TopicKind;
  ///
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let topic = domain_participant.create_topic("some_topic", "SomeType", &qos, TopicKind::WithKey);
  /// ```
  pub fn create_topic(
    &self,
    name: &str,
    type_desc: &str,
    qos: &QosPolicies,
    topic_kind: TopicKind,
  ) -> Result<Topic> {
    // println!("Create topic outer");
    let w = self.weak_clone();
    self
      .dpi
      .lock()
      .unwrap()
      .create_topic(&w, name, type_desc, qos, topic_kind)
  }

  pub fn find_topic(&self, name: &str, timeout: Duration) -> Result<Option<Topic>> {
    let w = self.weak_clone();
    self.dpi.lock().unwrap().find_topic(&w, name, timeout)
  }

  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let domain_id = domain_participant.domain_id();
  /// ```
  pub fn domain_id(&self) -> u16 {
    self.dpi.lock().unwrap().domain_id()
  }

  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let participant_id = domain_participant.participant_id();
  /// ```
  pub fn participant_id(&self) -> u16 {
    self.dpi.lock().unwrap().participant_id()
  }

  /// Gets all DiscoveredTopics from DDS network
  ///
  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let discovered_topics = domain_participant.get_discovered_topics();
  /// for dtopic in discovered_topics.iter() {
  ///   // do something
  /// }
  /// ```
  pub fn get_discovered_topics(&self) -> Vec<DiscoveredTopicData> {
    self.dpi.lock().unwrap().get_discovered_topics()
  }

  /// Manually asserts liveliness, affecting all writers with
  /// LIVELINESS QoS of MANUAL_BY_PARTICIPANT created by
  /// this particular participant.
  ///
  /// # Example
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// let domain_participant = DomainParticipant::new(0).expect("Failed to create participant");
  /// domain_participant.assert_liveliness();
  /// ```
  pub fn assert_liveliness(self) -> Result<()> {
    self.dpi.lock().unwrap().assert_liveliness()
  }

  pub(crate) fn weak_clone(&self) -> DomainParticipantWeak {
    DomainParticipantWeak::new(self.clone(), self.get_guid())
  }

  pub(crate) fn get_dds_cache(&self) -> Arc<RwLock<DDSCache>> {
    self.dpi.lock().unwrap().get_dds_cache()
  }

  pub(crate) fn discovery_db(&self) -> Arc<RwLock<DiscoveryDB>> {
    self
      .dpi
      .lock()
      .unwrap()
      .dpi
      .lock()
      .unwrap()
      .discovery_db
      .clone()
  }

}

impl PartialEq for DomainParticipant {
  fn eq(&self, other: &Self) -> bool {
    self.get_guid() == other.get_guid()
      && self.domain_id() == other.domain_id()
      && self.participant_id() == other.participant_id()
  }
}

#[derive(Clone)]
pub struct DomainParticipantWeak {
  dpi: Weak<Mutex<DomainParticipant_Disc>>,
  // This struct caches the GUID to avoid construction deadlocks
  guid: GUID,
}

impl DomainParticipantWeak {
  pub fn new(dp: DomainParticipant, guid: GUID) -> DomainParticipantWeak {
    DomainParticipantWeak {
      dpi: Arc::downgrade(&dp.dpi),
      guid,
    }
  }

  pub fn create_publisher(&self, qos: &QosPolicies) -> Result<Publisher> {
    match self.dpi.upgrade() {
      Some(dpi) => dpi.lock().unwrap().create_publisher(&self, qos),
      None => Err(Error::OutOfResources),
    }
  }

  pub fn create_subscriber<'a>(&self, qos: &QosPolicies) -> Result<Subscriber> {
    match self.dpi.upgrade() {
      Some(dpi) => dpi.lock().unwrap().create_subscriber(&self, qos),
      None => Err(Error::OutOfResources),
    }
  }

  pub fn create_topic(
    &self,
    name: &str,
    type_desc: &str,
    qos: &QosPolicies,
    topic_kind: TopicKind,
  ) -> Result<Topic> {
    match self.dpi.upgrade() {
      Some(dpi) => dpi
        .lock()
        .unwrap()
        .create_topic(&self, name, type_desc, qos, topic_kind),
      None => Err(Error::LockPoisoned),
    }
  }

  pub fn find_topic(&self, name: &str, timeout: Duration) -> Result<Option<Topic>> {
    match self.dpi.upgrade() {
      Some(dpi) => dpi.lock().unwrap().find_topic(&self, name, timeout),
      None => Err(Error::LockPoisoned),
    }
  }

  pub fn domain_id(&self) -> u16 {
    match self.dpi.upgrade() {
      Some(dpi) => dpi.lock().unwrap().domain_id(),
      None => panic!("Unable to get original domain participant."),
    }
  }

  pub fn participant_id(&self) -> u16 {
    match self.dpi.upgrade() {
      Some(dpi) => dpi.lock().unwrap().participant_id(),
      None => panic!("Unable to get original domain participant."),
    }
  }

  pub fn get_discovered_topics(&self) -> Vec<DiscoveredTopicData> {
    match self.dpi.upgrade() {
      Some(dpi) => dpi.lock().unwrap().get_discovered_topics(),
      None => Vec::new(),
    }
  }

  pub fn upgrade(self) -> Option<DomainParticipant> {
    match self.dpi.upgrade() {
      Some(d) => Some(DomainParticipant { dpi: d }),
      None => None,
    }
  }
} // end impl

impl RTPSEntity for DomainParticipantWeak {
  fn get_guid(&self) -> GUID {
    self.guid
  }
}

// This struct exists only to control and stop Discovery when DomainParticipant
// should be dropped
pub(crate) struct DomainParticipant_Disc {
  dpi: Arc<Mutex<DomainParticipant_Inner>>,
  // Discovery control
  discovery_updated_sender: Option<mio_channel::SyncSender<DiscoveryNotificationType>>,
  discovery_command_receiver: Option<mio_channel::Receiver<DiscoveryCommand>>,
  discovery_command_channel: mio_channel::SyncSender<DiscoveryCommand>,
  discovery_join_handle: mio_channel::Receiver<JoinHandle<()>>,
}

impl DomainParticipant_Disc {
  pub fn new(
    domain_id: u16,
    discovery_join_handle: mio_channel::Receiver<JoinHandle<()>>,
  ) -> Result<DomainParticipant_Disc> {
    let (discovery_update_notification_sender, discovery_update_notification_receiver) =
      mio_channel::sync_channel::<DiscoveryNotificationType>(100);

    let dpi = DomainParticipant_Inner::new(domain_id, discovery_update_notification_receiver)?;

    let (discovery_command_sender, discovery_command_receiver) =
      mio_channel::sync_channel::<DiscoveryCommand>(10);

    Ok(DomainParticipant_Disc {
      dpi: Arc::new(Mutex::new(dpi)),
      discovery_updated_sender: Some(discovery_update_notification_sender),
      discovery_command_receiver: Some(discovery_command_receiver),
      discovery_command_channel: discovery_command_sender,
      discovery_join_handle,
    })
  }

  pub fn create_publisher(
    &self,
    dp: &DomainParticipantWeak,
    qos: &QosPolicies,
  ) -> Result<Publisher> {
    self
      .dpi
      .lock()
      .unwrap()
      .create_publisher(&dp, qos, self.discovery_command_channel.clone())
  }

  pub fn create_subscriber<'a>(
    &self,
    dp: &DomainParticipantWeak,
    qos: &QosPolicies,
  ) -> Result<Subscriber> {
    self
      .dpi
      .lock()
      .unwrap()
      .create_subscriber(&dp, qos, self.discovery_command_channel.clone())
  }

  pub fn create_topic(
    &self,
    dp: &DomainParticipantWeak,
    name: &str,
    type_desc: &str,
    qos: &QosPolicies,
    topic_kind: TopicKind,
  ) -> Result<Topic> {
    // println!("Create topic disc");
    self
      .dpi
      .lock()
      .unwrap()
      .create_topic(&dp, name, type_desc, qos, topic_kind)
  }

  pub fn find_topic(
    &self,
    dp: &DomainParticipantWeak,
    name: &str,
    timeout: Duration,
  ) -> Result<Option<Topic>> {
    self.dpi.lock().unwrap().find_topic(&dp, name, timeout)
  }

  pub fn domain_id(&self) -> u16 {
    self.dpi.lock().unwrap().domain_id()
  }

  pub fn participant_id(&self) -> u16 {
    self.dpi.lock().unwrap().participant_id()
  }

  pub fn get_discovered_topics(&self) -> Vec<DiscoveredTopicData> {
    self.dpi.lock().unwrap().get_discovered_topics()
  }

  pub(crate) fn get_dds_cache(&self) -> Arc<RwLock<DDSCache>> {
    return self.dpi.lock().unwrap().get_dds_cache();
  }

  pub(crate) fn discovery_db(&self) -> Arc<RwLock<DiscoveryDB>> {
    return self.dpi.lock().unwrap().discovery_db.clone();
  }

  pub(crate) fn assert_liveliness(&self) -> Result<()> {
    // No point in checking for the LIVELINESS QoS of MANUAL_BY_PARTICIPANT,
    // the discovery command mutates a field which is only read
    // by writers with that particular QoS.
    self
      .discovery_command_channel
      .send(DiscoveryCommand::MANUAL_ASSERT_LIVELINESS)
      .or_else(|e| {
        log_and_err_internal!("assert_liveness - Failed to send DiscoveryCommand. {:?}", e)
      })
  }

}

impl Drop for DomainParticipant_Disc {
  fn drop(&mut self) {
    debug!("Sending Discovery Stop signal.");
    match self
      .discovery_command_channel
      .send(DiscoveryCommand::STOP_DISCOVERY)
    {
      Ok(_) => (),
      _ => {
        warn!("Failed to send stop signal to Discovery");
        return;
      }
    }

    debug!("Waiting for Discovery join.");
    if let Ok(handle) = self.discovery_join_handle.try_recv() {
      handle.join().unwrap();
      debug!("Joined Discovery.");
    }
  }
}

// This is the actual working DomainParticipant.
pub(crate) struct DomainParticipant_Inner {
  domain_id: u16,
  participant_id: u16,

  my_guid: GUID,

  // Adding Readers
  sender_add_reader: mio_channel::SyncSender<ReaderIngredients>,
  sender_remove_reader: mio_channel::SyncSender<GUID>,

  // Adding DataReaders
  sender_add_datareader_vec: Vec<mio_channel::SyncSender<()>>,
  sender_remove_datareader_vec: Vec<mio_channel::SyncSender<GUID>>,

  // dp_event_loop control
  stop_poll_sender: mio_channel::Sender<()>,
  ev_loop_handle: Option<JoinHandle<()>>, // this is Option, because it needs to be extracted
  // out of the struct (take) in order to .join() on the handle.

  // Writers
  add_writer_sender: mio_channel::SyncSender<Writer>,
  remove_writer_sender: mio_channel::SyncSender<GUID>,

  dds_cache: Arc<RwLock<DDSCache>>,
  discovery_db: Arc<RwLock<DiscoveryDB>>,

}

impl Drop for DomainParticipant_Inner {
  fn drop(&mut self) {
    // if send has an error simply leave as we have lost control of the
    // ev_loop_thread anyways
    match self.stop_poll_sender.send(()) {
      Ok(_) => (),
      _ => return (),
    };

    debug!("Waiting for dp_event_loop join");

    match self.ev_loop_handle.take() {
      Some(join_handle) => {
        join_handle
          .join()
          .unwrap_or_else(|e| warn!("Failed to join dp_event_loop: {:?}", e));
      }
      None => {
        error!("Someone managed to steal dp_event_loop join handle from DomainParticipant_Inner.");
      }
    }

    debug!("Joined dp_event_loop");
  }
}

#[allow(clippy::new_without_default)]
impl DomainParticipant_Inner {
  fn new(
    domain_id: u16,
    discovery_update_notification_receiver: mio_channel::Receiver<DiscoveryNotificationType>,
  ) -> Result<DomainParticipant_Inner> {
    let mut listeners = HashMap::new();

    match UDPListener::new_multicast(
        DISCOVERY_SENDER_TOKEN,
        "0.0.0.0",
        get_spdp_well_known_multicast_port(domain_id),
        Ipv4Addr::new(239, 255, 0, 1) ) 
    {
      Ok(l) => { listeners.insert(DISCOVERY_MUL_LISTENER_TOKEN, l); }
      Err(e) =>
        warn!("Cannot get multicast discovery listener: {:?}",e),
    }


    let mut participant_id = 0;

    let mut discovery_listener = None;

    // Magic value 120 below is from RTPS spec Section "9.6.1.3 Default Port Numbers"
    while discovery_listener.is_none() && participant_id < 120 {
      discovery_listener = UDPListener::new_unicast(
          DISCOVERY_SENDER_TOKEN,
          "0.0.0.0",
          get_spdp_well_known_unicast_port(domain_id, participant_id),
        ).ok();
      if discovery_listener.is_none() {
        participant_id += 1;
      }
    }

    info!("ParticipantId {} selected.", participant_id);

    // here discovery_listener is redefinde (shadowed)
    let discovery_listener = match discovery_listener {
      Some(dl) => dl,
      None => return log_and_err_internal!("Could not find free ParticipantId"),
    };
    listeners.insert(DISCOVERY_LISTENER_TOKEN, discovery_listener);

    // Now the user traffic listeners

    match UDPListener::new_multicast(
        USER_TRAFFIC_SENDER_TOKEN,
      "0.0.0.0",
      get_user_traffic_multicast_port(domain_id),
      Ipv4Addr::new(239, 255, 0, 1) )
    {
      Ok(l) => { listeners.insert(USER_TRAFFIC_MUL_LISTENER_TOKEN, l); }
      Err(e) =>
        warn!("Cannot get multicast discovery listener: {:?}",e),
    }


    let user_traffic_listener = UDPListener::new_unicast(
            USER_TRAFFIC_SENDER_TOKEN,
            "0.0.0.0",
            get_user_traffic_unicast_port(domain_id, participant_id),
          );
    let user_traffic_listener = match user_traffic_listener {
          Ok(l) => l,
          Err(e) => return log_and_err_internal!("Could not open user traffic listener: {:?}",e),
        };
    listeners.insert(USER_TRAFFIC_LISTENER_TOKEN, user_traffic_listener);

    // Adding readers
    let (sender_add_reader, receiver_add_reader) = mio_channel::sync_channel::<ReaderIngredients>(100);
    let (sender_remove_reader, receiver_remove_reader) = mio_channel::sync_channel::<GUID>(10);

    // Writers
    let (add_writer_sender, add_writer_receiver) = mio_channel::sync_channel::<Writer>(10);
    let (remove_writer_sender, remove_writer_receiver) = mio_channel::sync_channel::<GUID>(10);

    let new_guid = GUID::new_particiapnt_guid();
    let domain_info = DomainInfo {
      domain_participant_guid: new_guid,
      domain_id,
      participant_id,
    };

    let a_r_cache = Arc::new(RwLock::new(DDSCache::new()));

    let discovery_db = Arc::new(RwLock::new(DiscoveryDB::new(new_guid)));

    let (stop_poll_sender, stop_poll_receiver) = mio_channel::channel::<()>();

    // Launch the background thread for DomainParticipant
    let dds_cache_clone = a_r_cache.clone();
    let disc_db_clone = discovery_db.clone();
    let ev_loop_handle = thread::Builder::new()
      .name("RustDDS Participant event loop".to_string())
      .spawn(move || { 
        let ev_wrapper = DPEventLoop::new(
          domain_info,
          listeners,
          dds_cache_clone,
          disc_db_clone,
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
          discovery_update_notification_receiver,
        );
        ev_wrapper.event_loop()
      })?;

    info!("New DomainParticipant_Inner: domain_id={:?} participant_id={:?} GUID={:?}",
      domain_id, participant_id, new_guid);
    Ok(DomainParticipant_Inner {
      domain_id,
      participant_id,
      my_guid: new_guid,
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
      discovery_db,
    })
  }

  pub fn get_dds_cache(&self) -> Arc<RwLock<DDSCache>> {
    self.dds_cache.clone()
  }

  pub fn add_reader(&self, reader: ReaderIngredients) {
    self.sender_add_reader.send(reader).unwrap();
  }

  pub fn remove_reader(&self, guid: GUID) {
    let reader_guid = guid; // How to identify reader to be removed?
    self.sender_remove_reader.send(reader_guid).unwrap();
  }

  // Publisher and subscriber creation
  //
  // There are no delete function for publisher or subscriber. Deletion is
  // performed by deleting the Publisher or Subscriber object, who upon deletion
  // will notify the DomainParticipant.
  pub fn create_publisher(
    &self,
    domain_participant: &DomainParticipantWeak,
    qos: &QosPolicies,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  ) -> Result<Publisher> {
    Ok(Publisher::new(
      domain_participant.clone(),
      self.discovery_db.clone(),
      qos.clone(),
      qos.clone(),
      self.add_writer_sender.clone(),
      discovery_command,
    ))
  }

  pub fn create_subscriber(
    &self,
    domain_participant: &DomainParticipantWeak,
    qos: &QosPolicies,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  ) -> Result<Subscriber> {
    Ok(Subscriber::new(
      domain_participant.clone(),
      self.discovery_db.clone(),
      qos.clone(),
      self.sender_add_reader.clone(),
      self.sender_remove_reader.clone(),
      discovery_command,
    ))
  }

  // Topic creation. Data types should be handled as something (potentially) more
  // structured than a String. NOTE: Here we are using &str for topic name. &str
  // is Unicode string, whereas DDS specifes topic name to be a sequence of
  // octets, which would be &[u8] in Rust. This may cause problems if there are
  // topic names with non-ASCII characters. On the other hand, string handling
  // with &str is easier in Rust.
  pub fn create_topic(
    &self,
    domain_participant_weak: &DomainParticipantWeak,
    name: &str,
    type_desc: &str,
    qos: &QosPolicies,
    topic_kind: TopicKind,
  ) -> Result<Topic> {
    let topic = Topic::new(
      domain_participant_weak,
      name.to_string(),
      TypeDesc::new(type_desc),
      &qos,
      topic_kind,
    );
    Ok(topic)

    // TODO: refine
  }

  // Do not implement contentfilteredtopics or multitopics (yet)

  pub fn find_topic(
    &self,
    domain_participant_weak: &DomainParticipantWeak,
    name: &str,
    timeout: Duration,
  ) -> Result<Option<Topic>> {
    match self.find_topic_in_discovery_db(domain_participant_weak, name)? {
      Some(topic) => return Ok(Some(topic)),
      None => (),
    }

    std::thread::sleep(timeout);

    self.find_topic_in_discovery_db(domain_participant_weak, name)
  }

  fn find_topic_in_discovery_db(
    &self,
    domain_participant_weak: &DomainParticipantWeak,
    name: &str,
  ) -> Result<Option<Topic>> {
    let db = match self.discovery_db.read() {
      Ok(db) => db,
      Err(_) => return Err(Error::LockPoisoned),
    };

    let maybe_discovered_topic = db.get_all_topics().find(|&d| d.get_topic_name() == name);
    match maybe_discovered_topic {
      Some(d) => {
        // build a Topic from DiscoveredTopicData
        let qos = d.topic_data.get_qos();
        let topic_kind = match d.topic_data.key {
          Some(_) => TopicKind::WithKey,
          None => TopicKind::NoKey,
        };
        let name = d.get_topic_name().clone();
        let type_desc = d.topic_data.type_name.clone();
        let topic =
          self.create_topic(domain_participant_weak, &name, &type_desc, &qos, topic_kind)?;
        Ok(Some(topic))
      }
      None => Ok(None),
    }
  }
  // get_builtin_subscriber (why would we need this?)

  // ignore_* operations. TODO: Do we needa any of those?

  // delete_contained_entities is not needed. Data structures shoud be designed so
  // that lifetime of all created objects is within the lifetime of
  // DomainParticipant. Then such deletion is implicit.

  // The following methods are not for application use.

  pub(crate) fn get_add_reader_sender(&self) -> mio_channel::SyncSender<ReaderIngredients> {
    self.sender_add_reader.clone()
  }

  pub(crate) fn get_remove_reader_sender(&self) -> mio_channel::SyncSender<GUID> {
    self.sender_remove_reader.clone()
  }

  pub(crate) fn get_add_writer_sender(&self) -> mio_channel::SyncSender<Writer> {
    self.add_writer_sender.clone()
  }

  pub(crate) fn get_remove_writer_sender(&self) -> mio_channel::SyncSender<GUID> {
    self.remove_writer_sender.clone()
  }

  pub fn domain_id(&self) -> u16 {
    self.domain_id
  }

  pub fn participant_id(&self) -> u16 {
    self.participant_id
  }

  pub fn get_discovered_topics(&self) -> Vec<DiscoveredTopicData> {
    let db = match self.discovery_db.read() {
      Ok(db) => db,
      Err(e) => panic!("DiscoveryDB is poisoned. {:?}", e),
    };

    db.get_all_topics().map(|p| p.clone()).collect()
  }
} // impl

impl RTPSEntity for DomainParticipant {
  fn get_guid(&self) -> GUID {
    self.dpi.lock().unwrap().get_guid()
  }
}

impl RTPSEntity for DomainParticipant_Disc {
  fn get_guid(&self) -> GUID {
    self.dpi.lock().unwrap().get_guid()
  }
}

impl RTPSEntity for DomainParticipant_Inner {
  fn get_guid(&self) -> GUID {
    self.my_guid
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
  use std::{collections::BTreeSet, net::SocketAddr};
  use enumflags2::BitFlags;
  use log::info;
  use crate::{dds::topic::TopicKind, speedy::Writable};
  use crate::{
    dds::qos::QosPolicies,
    network::{udp_sender::UDPSender, constant::get_user_traffic_unicast_port},
    test::random_data::RandomData,
    structure::{
      locator::{LocatorKind, Locator},
      guid::{EntityId, GUID},
      sequence_number::{SequenceNumber, SequenceNumberSet},
    },
    messages::submessages::submessages::{
      EntitySubmessage, AckNack, SubmessageHeader, SubmessageKind,
    },
    serialization::{SubMessage, Message, submessage::*},
    messages::{
      protocol_version::ProtocolVersion, header::Header, vendor_id::VendorId,
      protocol_id::ProtocolId, submessages::submessages::*,
    },
  };
  use super::DomainParticipant;
  use speedy::Endianness;

  use crate::serialization::cdr_serializer::CDRSerializerAdapter;
  use byteorder::LittleEndian;

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
    let domain_participant = DomainParticipant::new(0).expect("Participant creation failed!");
    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos.clone())
      .expect("Failed to create publisher");

    let topic = domain_participant
      .create_topic("Aasii", "RandomData", &qos.clone(), TopicKind::WithKey)
      .expect("Failed to create topic");

    let mut _data_writer = publisher
      .create_datawriter::<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>>(topic, None)
      .expect("Failed to create datawriter");
  }

  #[test]
  fn dp_recieve_acknack_message_test() {
    // TODO SEND ACKNACK
    let domain_participant = DomainParticipant::new(0).expect("Failed to create participant");

    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();

    let publisher = domain_participant
      .create_publisher(&qos.clone())
      .expect("Failed to create publisher");

    let topic = domain_participant
      .create_topic("Aasii", "Huh?", &qos.clone(), TopicKind::WithKey)
      .expect("Failed to create topic");

    let mut _data_writer = publisher
      .create_datawriter::<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>>(topic, None)
      .expect("Failed to create datawriter");

    let portNumber: u16 = get_user_traffic_unicast_port(5, 0);
    let _sender = UDPSender::new(1234);
    let mut m: Message = Message::default();

    let a: AckNack = AckNack {
      reader_id: EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
      writer_id: EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
      reader_sn_state: SequenceNumberSet::from_base_and_set(
        SequenceNumber::default(),
        &BTreeSet::new(),
      ),
      count: 1,
    };
    let flags = BitFlags::<ACKNACK_Flags>::from_endianness(Endianness::BigEndian);
    let subHeader: SubmessageHeader = SubmessageHeader {
      kind: SubmessageKind::ACKNACK,
      flags: flags.bits(),
      content_length: 24,
    };

    let s: SubMessage = SubMessage {
      header: subHeader,
      body: SubmessageBody::Entity(EntitySubmessage::AckNack(a, flags)),
    };
    let h = Header {
      protocol_id: ProtocolId::default(),
      protocol_version: ProtocolVersion { major: 2, minor: 3 },
      vendor_id: VendorId::THIS_IMPLEMENTATION,
      guid_prefix: GUID::default().guidPrefix,
    };
    m.set_header(h);
    m.add_submessage(SubMessage::from(s));
    let _data: Vec<u8> = m.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap();
    info!("data to send via udp: {:?}", _data);
    let loca = Locator {
      kind: LocatorKind::LOCATOR_KIND_UDPv4,
      port: portNumber as u32,
      address: [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
      ],
      /* address: [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F,
       * 0x00, 0x00, 0x01], */
    };
    let locas = vec![loca];
    _sender.send_to_locator_list(&_data, &locas);
  }
}
