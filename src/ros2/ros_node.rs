use std::collections::{HashMap, HashSet};
use std::sync::{Arc,Mutex};
//use std::io::{Write,stderr};
use log::error;
use mio::Evented;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
  dds::{
    no_key::{
      datareader::DataReader as NoKeyDataReader, datawriter::DataWriter as NoKeyDataWriter,
    },
    DomainParticipant,
    pubsub::Publisher,
    pubsub::Subscriber,
    qos::QosPolicies,
    topic::{Topic, TopicKind},
    traits::key::Key,
    traits::key::Keyed,
    traits::serde_adapters::DeserializerAdapter,
    traits::serde_adapters::SerializerAdapter,
    values::result::Error,
    data_types::DiscoveredTopicData,
  },
  structure::{entity::Entity, guid::GUID},
};

use super::{
  KeyedRosPublisher, KeyedRosSubscriber, RosPublisher, RosSubscriber,
  builtin_datatypes::NodeInfo,
  builtin_datatypes::{Gid, Log, ParameterEvents, ROSParticipantInfo},
  builtin_topics::ParameterEventsTopic,
  builtin_topics::{ROSDiscoveryTopic, RosOutTopic},
};

// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------

/// [RosParticipant](struct.RosParticipant.html) sends and receives other participants information in ROS2 network
#[derive(Clone)]
pub struct RosParticipant {
  inner: Arc<Mutex<RosParticipantInner>>,
}

impl RosParticipant {
  pub fn new() -> Result<RosParticipant, Error> {
    let dp = DomainParticipant::new(0);
    Self::from_DomainParticipant(dp)
  }

  pub fn from_DomainParticipant(domain_participant: DomainParticipant) 
  -> Result<RosParticipant, Error> 
  {
    let i = RosParticipantInner::from_DomainParticipant(domain_participant)?;
    Ok( RosParticipant {
          inner: Arc::new( Mutex::new( i ) ),
        } )
  }
  /// Create a new ROS2 node
  pub fn new_RosNode(&self,name: &str, namespace: &str, options: NodeOptions) 
      -> Result<RosNode, Error> {
    RosNode::new(name,namespace,options,self.clone())
  }
  pub fn handle_node_read(&mut self) -> Vec<ROSParticipantInfo> {
    self.inner.lock().unwrap().handle_node_read()
  }
  /// Clears all nodes and updates our RosParticipantInfo to ROS2 network
  pub fn clear(&mut self) {
    self.inner.lock().unwrap().clear()
  }

  pub fn domain_id(&self) -> u16 {
    self.inner.lock().unwrap().domain_participant.domain_id()
  }

  pub fn get_discovered_topics(&self) -> Vec<DiscoveredTopicData> {
    self.domain_participant().get_discovered_topics()
  }

  pub fn add_node_info(&mut self, node_info: NodeInfo) {
    self.inner.lock().unwrap().add_node_info(node_info)
  }

  pub fn remove_node_info(&mut self, node_info: &NodeInfo) {
    self.inner.lock().unwrap().remove_node_info(node_info)
  }

  fn get_parameter_events_topic(&self) -> Topic {
    self.inner.lock().unwrap().ros_parameter_events_topic.clone()
  }

  fn get_rosout_topic(&self) -> Topic {
    self.inner.lock().unwrap().ros_rosout_topic.clone()
  }

  fn get_ros_discovery_publisher(&self) -> Publisher {
    self.inner.lock().unwrap().ros_discovery_publisher.clone()
  }

  fn get_ros_discovery_subscriber(&self) -> Subscriber {
    self.inner.lock().unwrap().ros_discovery_subscriber.clone()
  }

  fn domain_participant(&self) -> DomainParticipant {
    self.inner.lock().unwrap().domain_participant.clone()
  }

}

struct RosParticipantInner {
  nodes: HashMap<String, NodeInfo>,
  external_nodes: HashMap<Gid, Vec<NodeInfo>>,
  node_reader: NoKeyDataReader<ROSParticipantInfo>,
  node_writer: NoKeyDataWriter<ROSParticipantInfo>,

  domain_participant: DomainParticipant,
  ros_discovery_topic: Topic,
  ros_discovery_publisher: Publisher,
  ros_discovery_subscriber: Subscriber,

  ros_parameter_events_topic: Topic,
  ros_rosout_topic: Topic,
}

impl RosParticipantInner {

  // "new"
  pub fn from_DomainParticipant(domain_participant: DomainParticipant) 
  -> Result<RosParticipantInner, Error> 
  {

    let ros_discovery_topic = domain_participant.create_topic(
        ROSDiscoveryTopic::topic_name(),
        ROSDiscoveryTopic::type_name(),
        &ROSDiscoveryTopic::get_qos(),
        TopicKind::NoKey,
      )?;

    let ros_discovery_publisher =
      domain_participant.create_publisher(&ROSDiscoveryTopic::get_qos())?;
    let ros_discovery_subscriber =
      domain_participant.create_subscriber(&ROSDiscoveryTopic::get_qos())?;

    let ros_parameter_events_topic = domain_participant.create_topic(
      ParameterEventsTopic::topic_name(),
      ParameterEventsTopic::type_name(),
      &ParameterEventsTopic::get_qos(),
      TopicKind::NoKey,
    )?;

    let ros_rosout_topic = domain_participant.create_topic(
      RosOutTopic::topic_name(),
      RosOutTopic::type_name(),
      &RosOutTopic::get_qos(),
      TopicKind::NoKey,
    )?;

    let node_reader = ros_discovery_subscriber
      .create_datareader_no_key(ros_discovery_topic.clone(), None, None)?;

    let node_writer = ros_discovery_publisher
      .create_datawriter_no_key(None, ros_discovery_topic.clone(), None)?;

    Ok(RosParticipantInner {
      nodes: HashMap::new(),
      external_nodes: HashMap::new(),
      node_reader,
      node_writer,

      domain_participant,
      ros_discovery_topic,
      ros_discovery_publisher,
      ros_discovery_subscriber,
      ros_parameter_events_topic,
      ros_rosout_topic,
    })
  }

  /// Gets our current participant info we have sent to ROS2 network
  pub fn get_ros_participant_info(&self) -> ROSParticipantInfo {
    ROSParticipantInfo::new(
      Gid::from_guid(self.domain_participant.get_guid()),
      self.nodes.iter().map(|(_, p)| p.clone()).collect(),
    )
  }

  // Adds new NodeInfo and updates our RosParticipantInfo to ROS2 network
  fn add_node_info(&mut self, mut node_info: NodeInfo) {
    node_info.add_reader(Gid::from_guid(self.node_reader.get_guid()));
    node_info.add_writer(Gid::from_guid(self.node_writer.get_guid()));

    match self.nodes.insert(node_info.get_full_name(), node_info) {
      Some(_) => (),
      None => self.broadcast_node_infos(),
    }
  }

  /// Removes NodeInfo and updates our RosParticipantInfo to ROS2 network
  fn remove_node_info(&mut self, node_info: &NodeInfo) {
    match self.nodes.remove(&node_info.get_full_name()) {
      Some(_) => self.broadcast_node_infos(),
      None => (),
    }
  }

  /// Clears all nodes and updates our RosParticipantInfo to ROS2 network
  pub fn clear(&mut self) {
    if !self.nodes.is_empty() {
      self.nodes.clear();
      self.broadcast_node_infos()
    }
  }

  fn broadcast_node_infos(&self) {
    match self
      .node_writer
      .write(self.get_ros_participant_info(), None)
    {
      Ok(_) => (),
      Err(e) => error!("Failed to write into node_writer {:?}", e),
    }
  }

  /// Fetches all unread ROSParticipantInfos we have received
  pub fn handle_node_read(&mut self) -> Vec<ROSParticipantInfo> {
    let mut pts = Vec::new();
    while let Ok(Some(sample)) = self.node_reader.take_next_sample() {
      let rpi = sample.value();
      match self.external_nodes.get_mut(&rpi.guid()) {
        Some(rpi2) => {
          *rpi2 = rpi.nodes().to_vec();
        }
        None => {
          self.external_nodes.insert(rpi.guid(), rpi.nodes().to_vec());
        }
      };
      pts.push(rpi.clone());
    }
    pts
  }

}

impl Evented for RosParticipant {
  fn register(&self, poll: &mio::Poll, token: mio::Token, interest: mio::Ready, opts: mio::PollOpt, ) 
  -> std::io::Result<()>  {
    poll.register(&self.inner.lock().unwrap().node_reader, token, interest, opts)
  }

  fn reregister(&self, poll: &mio::Poll, token: mio::Token, interest: mio::Ready, opts: mio::PollOpt, ) 
    -> std::io::Result<()>  {
    poll.reregister(&self.inner.lock().unwrap().node_reader, token, interest, opts)
  }

  fn deregister(&self, poll: &mio::Poll) -> std::io::Result<()> {
    poll.deregister(&self.inner.lock().unwrap().node_reader)
  }
}

// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------

/// Configuration of [RosNode](struct.RosNode.html)
pub struct NodeOptions {
  enable_rosout: bool,
}

impl NodeOptions {
  /// # Arguments
  ///
  /// * `enable_rosout` -  Wheter or not ros logging is enabled (rosout writer)
  pub fn new(/*domain_id: u16, */enable_rosout: bool) -> NodeOptions {
    NodeOptions {
      enable_rosout,
    }
  }
}

// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------

/// Node in ROS2 network. Holds necessary readers and writers for rosout and parameter events topics internally.
/// Should be constructed using [builder](struct.RosNodeBuilder.html).
pub struct RosNode {
  // node info
  name: String,
  namespace: String,
  options: NodeOptions,

  ros_participant: RosParticipant,

  // dynamic
  readers: HashSet<GUID>,
  writers: HashSet<GUID>,

  // builtin writers and readers
  rosout_writer: Option<NoKeyDataWriter<Log>>,
  rosout_reader: Option<NoKeyDataReader<Log>>,
  parameter_events_writer: NoKeyDataWriter<ParameterEvents>,
}

impl RosNode {
  fn new(
    name: &str,
    namespace: &str,
    options: NodeOptions,
    ros_participant: RosParticipant,
  ) -> Result<RosNode, Error> {
    let paramtopic = ros_participant.get_parameter_events_topic();
    let rosout_topic = ros_participant.get_rosout_topic();

    let rosout_writer = if options.enable_rosout {
      Some(
        ros_participant
          .get_ros_discovery_publisher()
          .create_datawriter_no_key(None, rosout_topic.clone(), None)?,
      )
    } else {
      None
    };

    let parameter_events_writer = ros_participant
      .get_ros_discovery_publisher()
      .create_datawriter_no_key(None, paramtopic.clone(), None)?;

    Ok(RosNode {
      name: String::from(name),
      namespace: String::from(namespace),
      options,
      ros_participant,
      readers: HashSet::new(),
      writers: HashSet::new(),
      rosout_writer,
      rosout_reader: None,
      parameter_events_writer,
    })
  }

  // Generates ROS2 node info from added readers and writers.
  fn generate_node_info(&self) -> NodeInfo {
    let mut node_info = NodeInfo::new(self.name.to_owned(), self.namespace.to_owned());

    node_info.add_writer( Gid::from_guid(self.parameter_events_writer.get_guid()) );
    if let Some(row) = &self.rosout_writer {
      node_info.add_writer( Gid::from_guid(row.get_guid()) )
    }

    for reader in self.readers.iter() {
      node_info.add_reader( Gid::from_guid(*reader) )
    }

    for writer in self.writers.iter() {
      node_info.add_writer( Gid::from_guid(*writer) );
    }

    node_info
  }

  fn add_reader(&mut self, reader: GUID) {
    self.readers.insert(reader);
    self.ros_participant.add_node_info( self.generate_node_info() );
  }

  pub fn remove_reader(&mut self, reader: &GUID) {
    self.readers.remove(reader);
    self.ros_participant.add_node_info( self.generate_node_info() );
  }

  fn add_writer(&mut self, writer: GUID) {
    self.writers.insert(writer);
    self.ros_participant.add_node_info( self.generate_node_info() );
  }

  pub fn remove_writer(&mut self, writer: &GUID) {
    self.writers.remove(writer);
    self.ros_participant.add_node_info( self.generate_node_info() );
  }

  /// Clears both all reader and writer guids from this node.
  pub fn clear_node(&mut self) {
    self.readers.clear();
    self.writers.clear();
    self.ros_participant.add_node_info( self.generate_node_info() );
  }

  pub fn get_name(&self) -> &str {
    &self.name
  }

  pub fn get_namespace(&self) -> &str {
    &self.namespace
  }

  pub fn get_fully_qualified_name(&self) -> String {
    let mut nn = self.name.clone();
    nn.push_str(&self.namespace);
    nn
  }

  pub fn get_options(&self) -> &NodeOptions {
    &self.options
  }

  pub fn get_domain_id(&self) -> u16 {
    self.ros_participant.domain_id()
  }

  /// Creates ROS2 topic and handles necessary conversions from DDS to ROS2
  ///
  /// # Arguments
  ///
  /// * `domain_participant` - [DomainParticipant](../dds/struct.DomainParticipant.html)
  /// * `name` - Name of the topic
  /// * `type_name` - What type the topic holds in string form
  /// * `qos` - Quality of Service parameters for the topic (not restricted only to ROS2)
  /// * `topic_kind` - Does the topic have a key (multiple DDS instances)? NoKey or WithKey
  pub fn create_ros_topic(&self,
    name: &str,
    type_name: &str,
    qos: QosPolicies,
    topic_kind: TopicKind,
  ) -> Result<Topic, Error> {
    // TODO:
    // Implement according to the rules in
    // https://design.ros2.org/articles/topic_and_service_names.html
    let mut oname = "rt/".to_owned();
    let name_stripped = name.strip_prefix("/").unwrap_or(name); // avoid double slash in name
    oname.push_str(name_stripped);
    println!("Crete topic, DDS name: {}",oname);
    let topic = self.ros_participant.domain_participant()
      .create_topic(&oname, type_name, &qos, topic_kind)?;
    Ok(topic)
  }

  /// Creates ROS2 Subscriber to no key topic.
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to topic created with `create_ros_topic`.
  /// * `qos` - Should take [QOS](../dds/qos/struct.QosPolicies.html) and use if it's compatible with topics QOS. `None` indicates the use of Topics QOS.
  pub fn create_ros_nokey_subscriber<D: DeserializeOwned + 'static, DA: DeserializerAdapter<D>>(
    &mut self,
    topic: Topic,
    qos: Option<QosPolicies>,
  ) -> Result<RosSubscriber<D, DA>, Error> {
    let sub =
      self
        .ros_participant
        .get_ros_discovery_subscriber()
        .create_datareader_no_key::<D, DA>(topic, None, qos)?;
    self.add_reader( sub.get_guid() );
    Ok( sub )
  }

  /// Creates ROS2 Subscriber to [Keyed](../dds/traits/trait.Keyed.html) topic.
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to topic created with `create_ros_topic`.
  /// * `qos` - Should take [QOS](../dds/qos/struct.QosPolicies.html) and use it if it's compatible with topics QOS. `None` indicates the use of Topics QOS.
  pub fn create_ros_subscriber<D, DA: DeserializerAdapter<D>>(
    &mut self,
    topic: Topic,
    qos: Option<QosPolicies>,
  ) -> Result<KeyedRosSubscriber<D, DA>, Error>
  where
    D: Keyed + DeserializeOwned + 'static,
    D::K: Key,
  {
    let sub = self
      .ros_participant
      .get_ros_discovery_subscriber()
      .create_datareader::<D, DA>(topic, None, qos)?;
    self.add_reader( sub.get_guid() );
    Ok( sub )     
  }

  /// Creates ROS2 Publisher to no key topic.
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to topic created with `create_ros_topic`.
  /// * `qos` - Should take [QOS](../dds/qos/struct.QosPolicies.html) and use it if it's compatible with topics QOS. `None` indicates the use of Topics QOS.
  pub fn create_ros_nokey_publisher<D: Serialize, SA: SerializerAdapter<D>>(
    &mut self,
    topic: Topic,
    qos: Option<QosPolicies>,
  ) -> Result<RosPublisher<D, SA>, Error> {
    let p = self
      .ros_participant
      .get_ros_discovery_publisher()
      .create_datawriter_no_key(None, topic, qos)?;
    self.add_writer( p.get_guid() );
    Ok(p)
  }

  /// Creates ROS2 Publisher to [Keyed](../dds/traits/trait.Keyed.html) topic.
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to topic created with `create_ros_topic`.
  /// * `qos` - Should take [QOS](../dds/qos/struct.QosPolicies.html) and use it if it's compatible with topics QOS. `None` indicates the use of Topics QOS.
  pub fn create_ros_publisher<D, SA: SerializerAdapter<D>>(
    &mut self,
    topic: Topic,
    qos: Option<QosPolicies>,
  ) -> Result<KeyedRosPublisher<D, SA>, Error>
  where
    D: Keyed + Serialize ,
    D::K: Key,
  {
    let p = self
      .ros_participant
      .get_ros_discovery_publisher()
      .create_datawriter(None, topic, qos)?;
    self.add_writer( p.get_guid() );
    Ok(p)
  }
}
