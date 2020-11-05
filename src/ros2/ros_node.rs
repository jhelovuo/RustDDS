use std::collections::{HashMap, HashSet};

use byteorder::LittleEndian;
use log::error;
use mio::Evented;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
  dds::{
    //interfaces::IDataReader,
    //interfaces::IDataWriter,
    //interfaces::IKeyedDataReader,
    //interfaces::IKeyedDataWriter,
    no_key::{
      datareader::DataReader as NoKeyDataReader, datawriter::DataWriter as NoKeyDataWriter,
    },
    DomainParticipant,
    pubsub::Publisher,
    pubsub::Subscriber,
    qos::QosPolicies,
    topic::Topic,
    traits::key::Key,
    traits::key::Keyed,
    traits::serde_adapters::DeserializerAdapter,
    traits::serde_adapters::SerializerAdapter,
    values::result::Error,
  },
  serialization::cdr_deserializer::CDRDeserializerAdapter,
  serialization::cdr_serializer::CDRSerializerAdapter,
  structure::{entity::Entity, guid::GUID},
};

use super::{
  KeyedRosPublisher, KeyedRosSubscriber, RosPublisher, RosSubscriber,
  builtin_datatypes::NodeInfo,
  builtin_datatypes::{Gid, Log, ParameterEvents, ROSParticipantInfo},
  builtin_topics::ParameterEventsTopic,
  builtin_topics::{ROSDiscoveryTopic, RosOutTopic},
};

/// Trait for RCL interface
pub trait IRosNode {
  /// Name of the node
  fn get_name(&self) -> &str;
  /// Namespace node belongs to
  fn get_namespace(&self) -> &str;
  /// Namespace + Name (use name and namespace functions when possible)
  fn get_fully_qualified_name(&self) -> String;
  /// Nodes options when node has been initialized
  fn get_options(&self) -> &NodeOptions;
  /// DomainParticipants domain_id
  fn get_domain_id(&self) -> u16;
}

/// Trait for necessary DDS interface functions
pub trait IRosNodeControl<'a> {
  /// Creates ROS2 topic and handles necessary conversions from DDS to ROS2
  ///
  /// # Arguments
  ///
  /// * `domain_participant` - [DomainParticipant](../dds/struct.DomainParticipant.html)
  /// * `name` - Name of the topic
  /// * `type_name` - What type the topic holds in string form
  /// * `qos` - Quality of Service parameters for the topic (not restricted only to ROS2)
  fn create_ros_topic(
    domain_participant: &DomainParticipant,
    name: &str,
    type_name: &str,
    qos: QosPolicies,
  ) -> Result<Topic, Error>;

  /// Creates ROS2 Subscriber to no key topic.
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to topic created with `create_ros_topic`.
  /// * `qos` - Should take [QOS](../dds/qos/struct.QosPolicies.html) and use if it's compatible with topics QOS. `None` indicates the use of Topics QOS.
  fn create_ros_nokey_subscriber<D: DeserializeOwned + 'static, DA: DeserializerAdapter<D> + 'a>(
    &mut self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<RosSubscriber<'a, D, DA>, Error>;

  /// Creates ROS2 Subscriber to [Keyed](../dds/traits/trait.Keyed.html) topic.
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to topic created with `create_ros_topic`.
  /// * `qos` - Should take [QOS](../dds/qos/struct.QosPolicies.html) and use it if it's compatible with topics QOS. `None` indicates the use of Topics QOS.
  fn create_ros_subscriber<D, DA: DeserializerAdapter<D> + 'a>(
    &self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<KeyedRosSubscriber<'a, D, DA>, Error>
  where
    D: Keyed + DeserializeOwned + 'static,
    D::K: Key;

  /// Creates ROS2 Publisher to no key topic.
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to topic created with `create_ros_topic`.
  /// * `qos` - Should take [QOS](../dds/qos/struct.QosPolicies.html) and use it if it's compatible with topics QOS. `None` indicates the use of Topics QOS.
  fn create_ros_nokey_publisher<D: Serialize + 'a, SA: SerializerAdapter<D> + 'a>(
    &self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<RosPublisher<'a, D, SA>, Error>;

  /// Creates ROS2 Publisher to [Keyed](../dds/traits/trait.Keyed.html) topic.
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to topic created with `create_ros_topic`.
  /// * `qos` - Should take [QOS](../dds/qos/struct.QosPolicies.html) and use it if it's compatible with topics QOS. `None` indicates the use of Topics QOS.
  fn create_ros_publisher<D, SA: SerializerAdapter<D> + 'a>(
    &self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<KeyedRosPublisher<'a, D, SA>, Error>
  where
    D: Keyed + Serialize + 'a,
    D::K: Key;
}

/// [RosParticipant](struct.RosParticipant.html) sends and receives other participants information in ROS2 network
pub struct RosParticipant<'a> {
  nodes: HashMap<String, NodeInfo>,
  external_nodes: HashMap<Gid, Vec<NodeInfo>>,
  node_reader: NoKeyDataReader<'a, ROSParticipantInfo, CDRDeserializerAdapter<ROSParticipantInfo>>,
  node_writer:
    NoKeyDataWriter<'a, ROSParticipantInfo, CDRSerializerAdapter<ROSParticipantInfo, LittleEndian>>,
  ros_context: &'a RosContext,
}

impl<'a> RosParticipant<'a> {
  pub fn new(ros_context: &'a RosContext) -> Result<RosParticipant<'a>, Error> {
    let dtopic = match ros_context.get_ros_discovery_topic() {
      Some(t) => t,
      None => {
        error!("RosContext has no discovery topic.");
        return Err(Error::PreconditionNotMet);
      }
    };

    let node_reader = ros_context
      .get_ros_discovery_subscriber()
      .create_datareader_no_key(dtopic, None,  None)?;

    let node_writer = ros_context
      .get_ros_discovery_publisher()
      .create_datawriter_no_key(None, dtopic, None)?;

    Ok(RosParticipant {
      nodes: HashMap::new(),
      external_nodes: HashMap::new(),
      node_reader,
      node_writer,
      ros_context,
    })
  }

  /// Gets our current participant info we have sent to ROS2 network
  pub fn get_ros_participant_info(&self) -> ROSParticipantInfo {
    ROSParticipantInfo::new(
      Gid::from_guid(self.ros_context.domain_participant.get_guid()),
      self.nodes.iter().map(|(_, p)| p.clone()).collect(),
    )
  }

  /// Adds new NodeInfo and updates our RosParticipantInfo to ROS2 network
  pub fn add_node_info(&mut self, mut node_info: NodeInfo) {
    node_info.add_reader(Gid::from_guid(self.node_reader.get_guid()));
    node_info.add_writer(Gid::from_guid(self.node_writer.get_guid()));

    match self.nodes.insert(node_info.get_full_name(), node_info) {
      Some(_) => (),
      None => self.write_info(),
    }
  }

  /// Removes NodeInfo and updates our RosParticipantInfo to ROS2 network
  pub fn remove_node_info(&mut self, node_info: &NodeInfo) {
    match self.nodes.remove(&node_info.get_full_name()) {
      Some(_) => self.write_info(),
      None => (),
    }
  }

  /// Clears all nodes and updates our RosParticipantInfo to ROS2 network
  pub fn clear(&mut self) {
    if !self.nodes.is_empty() {
      self.nodes.clear();
      self.write_info()
    }
  }

  fn write_info(&self) {
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
      match sample.get_value() {
        Some(rpi) => {
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
        None => (),
      };
    }
    pts
  }
}

/// Is a helper for keeping lifetimes in check.
/// In addition, [RosContext](struct.RosContext.html) holds DDS
/// Publisher and Subscriber for creating readers and writers.
/// There has to be a [RosContext](struct.RosContext.html) for each
/// thread and they cannot be shared between threads.
pub struct RosContext {
  domain_participant: DomainParticipant,
  ros_discovery_topic: Option<Topic>,
  ros_discovery_publisher: Publisher,
  ros_discovery_subscriber: Subscriber,
  ros_parameter_events_topic: Topic<ParameterEvents>,
  ros_rosout_topic: Topic<Log>,
}

impl RosContext {
  /// # Arguments
  ///
  /// * `domain_participant` - DDS [DomainParticipant](../dds/struct.DomainParticipant.html)
  /// * `is_ros_participant_thread` - Information if this is the thread
  /// where our [RosParticipant](struct.RosParticipant.html) has been defined.
  pub fn new(
    domain_participant: DomainParticipant,
    is_ros_participant_thread: bool,
  ) -> Result<RosContext, Error> {
    let ros_discovery_topic = if is_ros_participant_thread {
      Some(domain_participant.create_topic(
        ROSDiscoveryTopic::topic_name(),
        ROSDiscoveryTopic::type_name(),
        &ROSDiscoveryTopic::get_qos(),
      )?)
    } else {
      None
    };

    let ros_discovery_publisher =
      domain_participant.create_publisher(&ROSDiscoveryTopic::get_qos())?;
    let ros_discovery_subscriber =
      domain_participant.create_subscriber(&ROSDiscoveryTopic::get_qos())?;

    let ros_parameter_events_topic = domain_participant.create_topic(
      ParameterEventsTopic::topic_name(),
      ParameterEventsTopic::type_name(),
      &ParameterEventsTopic::get_qos(),
    )?;

    let ros_rosout_topic = domain_participant.create_topic(
      RosOutTopic::topic_name(),
      RosOutTopic::type_name(),
      &RosOutTopic::get_qos(),
    )?;

    Ok(RosContext {
      domain_participant,
      ros_discovery_topic,
      ros_discovery_publisher,
      ros_discovery_subscriber,
      ros_parameter_events_topic,
      ros_rosout_topic,
    })
  }

  pub(crate) fn get_ros_discovery_topic(&self) -> Option<&Topic> {
    self.ros_discovery_topic.as_ref()
  }

  pub(crate) fn get_ros_discovery_subscriber(&self) -> &Subscriber {
    &self.ros_discovery_subscriber
  }

  pub(crate) fn get_ros_discovery_publisher(&self) -> &Publisher {
    &self.ros_discovery_publisher
  }

  pub(crate) fn get_parameter_events_topic(&self) -> &Topic {
    &self.ros_parameter_events_topic
  }

  pub(crate) fn get_rosout_topic(&self) -> &Topic {
    &self.ros_rosout_topic
  }
}

/// Configuration of [RosNode](struct.RosNode.html)
pub struct NodeOptions {
  domain_id: u16,
  enable_rosout: bool,
}

impl NodeOptions {
  /// # Arguments
  ///
  /// * `domain_id` - DomainParticipants domain_id
  /// * `enable_rosout` -  Wheter or not ros logging is enabled (rosout writer)
  pub fn new(domain_id: u16, enable_rosout: bool) -> NodeOptions {
    NodeOptions {
      domain_id,
      enable_rosout,
    }
  }
}

/// Builder for [RosNodes](struct.RosNode.html)
pub struct RosNodeBuilder<'a> {
  name: Option<String>,
  namespace: Option<String>,
  node_options: Option<NodeOptions>,
  // lifetime bound
  ros_context: Option<&'a RosContext>,
}

impl<'a> RosNodeBuilder<'a> {
  pub fn new() -> RosNodeBuilder<'a> {
    RosNodeBuilder {
      name: None,
      namespace: None,
      node_options: None,
      ros_context: None,
    }
  }

  /// <b>Required</b>.
  pub fn name(mut self, name: &str) -> RosNodeBuilder<'a> {
    self.name = Some(name.to_string());
    self
  }

  /// <b>Required</b>.
  pub fn namespace(mut self, namespace: &str) -> RosNodeBuilder<'a> {
    self.namespace = Some(namespace.to_string());
    self
  }

  /// Optional. When building defaults to `domain_id = 0` and `enable_rosout = false`
  pub fn node_options(mut self, node_options: NodeOptions) -> RosNodeBuilder<'a> {
    self.node_options = Some(node_options);
    self
  }

  /// <b>Required</b>.
  pub fn ros_context(mut self, ros_context: &'a RosContext) -> RosNodeBuilder<'a> {
    self.ros_context = Some(ros_context);
    self
  }

  pub fn build(self) -> Result<RosNode<'a>, Error> {
    let name = match self.name {
      Some(n) => n,
      None => {
        error!("Node name needs to be defined.");
        return Err(Error::PreconditionNotMet);
      }
    };

    let namespace = match self.namespace {
      Some(ns) => ns,
      None => {
        error!("Node namespace needs to be defined.");
        return Err(Error::PreconditionNotMet);
      }
    };

    let node_options = match self.node_options {
      Some(no) => no,
      None => NodeOptions::new(0, false),
    };

    let ros_context = match self.ros_context {
      Some(ctx) => ctx,
      None => {
        error!("RosContext needs to be defined.");
        return Err(Error::PreconditionNotMet);
      }
    };

    RosNode::new(&name, &namespace, node_options, ros_context)
  }
}

/// Node in ROS2 network. Holds necessary readers and writers for rosout and parameter events topics internally.
/// Should be constructed using [builder](struct.RosNodeBuilder.html).
pub struct RosNode<'a> {
  // node info
  name: String,
  namespace: String,
  options: NodeOptions,

  ros_context: &'a RosContext,

  // dynamic
  readers: HashSet<GUID>,
  writers: HashSet<GUID>,

  // builtin writers and readers
  rosout_writer: Option<NoKeyDataWriter<'a, Log, CDRSerializerAdapter<Log>>>,
  rosout_reader: Option<NoKeyDataReader<'a, Log, CDRDeserializerAdapter<Log>>>,
  parameter_events_writer:
    NoKeyDataWriter<'a, ParameterEvents, CDRSerializerAdapter<ParameterEvents>>,
}

impl<'a> RosNode<'a> {
  pub(crate) fn new(
    name: &str,
    namespace: &str,
    options: NodeOptions,
    ros_context: &'a RosContext,
  ) -> Result<RosNode<'a>, Error> {
    let paramtopic = ros_context.get_parameter_events_topic();
    let rosout_topic = ros_context.get_rosout_topic();

    let rosout_writer = if options.enable_rosout {
      Some(
        ros_context
          .get_ros_discovery_publisher()
          .create_datawriter_no_key(None, rosout_topic, None)?,
      )
    } else {
      None
    };

    let parameter_events_writer = ros_context
      .get_ros_discovery_publisher()
      .create_datawriter_no_key(None, paramtopic, None)?;

    Ok(RosNode {
      name: String::from(name),
      namespace: String::from(namespace),
      options,
      ros_context,
      readers: HashSet::new(),
      writers: HashSet::new(),
      rosout_writer,
      rosout_reader: None,
      parameter_events_writer,
    })
  }

  /// Generates ROS2 node info from added readers and writers.
  pub fn generate_node_info(&self) -> NodeInfo {
    let mut reader_guid = vec![];
    let mut writer_guid = vec![Gid::from_guid(self.parameter_events_writer.get_guid())];

    match &self.rosout_writer {
      Some(w) => writer_guid.push(Gid::from_guid(w.get_guid())),
      None => (),
    };

    for reader in self.readers.iter() {
      reader_guid.push(Gid::from_guid(*reader));
    }

    for writer in self.writers.iter() {
      writer_guid.push(Gid::from_guid(*writer));
    }

    let mut node_info = NodeInfo::new(self.name.to_owned(), self.namespace.to_owned());

    for reader_gid in reader_guid.into_iter() {
      node_info.add_reader(reader_gid);
    }

    for writer_gid in writer_guid.into_iter() {
      node_info.add_writer(writer_gid);
    }

    node_info
  }

  pub fn add_reader(&mut self, reader: GUID) {
    self.readers.insert(reader);
  }

  pub fn remove_reader(&mut self, reader: &GUID) {
    self.readers.remove(reader);
  }

  pub fn add_writer(&mut self, writer: GUID) {
    self.writers.insert(writer);
  }

  pub fn remove_writer(&mut self, writer: &GUID) {
    self.writers.remove(writer);
  }

  /// Clears both all reader and writer guids from this node.
  pub fn clear_node(&mut self) {
    self.readers.clear();
    self.writers.clear();
  }
}

impl IRosNode for RosNode<'_> {
  fn get_name(&self) -> &str {
    &self.name
  }

  fn get_namespace(&self) -> &str {
    &self.namespace
  }

  fn get_fully_qualified_name(&self) -> String {
    let mut nn = self.name.clone();
    nn.push_str(&self.namespace);
    nn
  }

  fn get_options(&self) -> &NodeOptions {
    &self.options
  }

  fn get_domain_id(&self) -> u16 {
    self.options.domain_id
  }
}

impl Evented for RosParticipant<'_> {
  fn register(
    &self,
    poll: &mio::Poll,
    token: mio::Token,
    interest: mio::Ready,
    opts: mio::PollOpt,
  ) -> std::io::Result<()> {
    poll.register(&self.node_reader, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &mio::Poll,
    token: mio::Token,
    interest: mio::Ready,
    opts: mio::PollOpt,
  ) -> std::io::Result<()> {
    poll.reregister(&self.node_reader, token, interest, opts)
  }

  fn deregister(&self, poll: &mio::Poll) -> std::io::Result<()> {
    poll.deregister(&self.node_reader)
  }
}

impl<'a> IRosNodeControl<'a> for RosNode<'a> {
  fn create_ros_topic(
    domain_participant: &DomainParticipant,
    name: &str,
    type_name: &str,
    qos: QosPolicies,
  ) -> Result<Topic, Error> {
    let mut oname = "rt".to_owned();
    oname.push_str(name);
    let topic = domain_participant.create_topic(&oname, type_name, &qos)?;
    Ok(topic)
  }

  fn create_ros_nokey_subscriber<D: DeserializeOwned + 'static, DA: DeserializerAdapter<D> + 'a>(
    &mut self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<Box<dyn IDataReader<D, DA> + 'a>, Error> {
    match self
      .ros_context
      .get_ros_discovery_subscriber()
      .create_datareader_no_key::<D, DA>(topic, None,  qos)
    {
      Ok(dr) => Ok(Box::new(dr)),
      Err(e) => Err(e),
    }
  }

  fn create_ros_subscriber<D, DA: DeserializerAdapter<D> + 'a>(
    &self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<Box<dyn IKeyedDataReader<D, DA> + 'a>, Error>
  where
    D: Keyed + DeserializeOwned + 'static,
    D::K: Key,
  {
    match self
      .ros_context
      .get_ros_discovery_subscriber()
      .create_datareader::<D, DA>(topic, None,  qos)
    {
      Ok(dr) => Ok(Box::new(dr)),
      Err(e) => Err(e),
    }
  }

  fn create_ros_nokey_publisher<D: Serialize + 'a, SA: SerializerAdapter<D> + 'a>(
    &self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<Box<dyn IDataWriter<D, SA> + 'a>, Error> {
    match self
      .ros_context
      .get_ros_discovery_publisher()
      .create_datawriter_no_key(None, topic, qos)
    {
      Ok(dw) => Ok(Box::new(dw)),
      Err(e) => Err(e),
    }
  }

  fn create_ros_publisher<D, SA: SerializerAdapter<D> + 'a>(
    &self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<Box<dyn IKeyedDataWriter<D, SA> + 'a>, Error>
  where
    D: Keyed + Serialize + 'a,
    D::K: Key,
  {
    match self
      .ros_context
      .get_ros_discovery_publisher()
      .create_datawriter(None, topic, qos)
    {
      Ok(dw) => Ok(Box::new(dw)),
      Err(e) => Err(e),
    }
  }
}
