use std::collections::{HashMap, HashSet};

use byteorder::LittleEndian;
use log::error;
use mio::Evented;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
  dds::{
    interfaces::IDataReader,
    interfaces::IDataWriter,
    interfaces::IKeyedDataReader,
    interfaces::IKeyedDataWriter,
    no_key::{
      datareader::DataReader as NoKeyDataReader, datawriter::DataWriter as NoKeyDataWriter,
    },
    participant::DomainParticipant,
    pubsub::Publisher,
    pubsub::Subscriber,
    qos::QosPolicies,
    topic::Topic,
    traits::key::Key,
    traits::key::Keyed,
    traits::serde_adapters::DeserializerAdapter,
    traits::serde_adapters::SerializerAdapter,
    typedesc::TypeDesc,
    values::result::Error,
  },
  serialization::cdrDeserializer::CDR_deserializer_adapter,
  serialization::cdrSerializer::CDR_serializer_adapter,
  structure::{entity::Entity, guid::GUID},
};

use super::{
  builtin_datatypes::NodeInfo,
  builtin_datatypes::{Gid, Log, ParameterEvents, ROSParticipantInfo},
  builtin_topics::ParameterEventsTopic,
  builtin_topics::{ROSDiscoveryTopic, RosOutTopic},
};

pub trait IRosNode {
  fn get_name(&self) -> &str;
  fn get_namespace(&self) -> &str;
  fn get_fully_qualified_name(&self) -> String;
  fn get_options(&self) -> &NodeOptions;
  fn get_domain_id(&self) -> u16;
}

pub trait IRosNodeControl<'a> {
  fn create_ros_topic(
    domain_participant: &DomainParticipant,
    name: &str,
    type_name: &str,
    qos: QosPolicies,
  ) -> Result<Topic, Error>;

  fn create_ros_nokey_subscriber<D: DeserializeOwned + 'static, DA: DeserializerAdapter<D> + 'a>(
    &mut self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<Box<dyn IDataReader<D, DA> + 'a>, Error>;

  fn create_ros_subscriber<D, DA: DeserializerAdapter<D> + 'a>(
    &self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<Box<dyn IKeyedDataReader<D, DA> + 'a>, Error>
  where
    D: Keyed + DeserializeOwned + 'static,
    D::K: Key;

  fn create_ros_nokey_publisher<D: Serialize + 'a, SA: SerializerAdapter<D> + 'a>(
    &self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<Box<dyn IDataWriter<D, SA> + 'a>, Error>;

  fn create_ros_publisher<D, SA: SerializerAdapter<D> + 'a>(
    &self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<Box<dyn IKeyedDataWriter<D, SA> + 'a>, Error>
  where
    D: Keyed + Serialize + 'a,
    D::K: Key;
}

pub struct RosParticipant<'a> {
  nodes: HashMap<String, NodeInfo>,
  external_nodes: HashMap<Gid, Vec<NodeInfo>>,
  node_reader:
    NoKeyDataReader<'a, ROSParticipantInfo, CDR_deserializer_adapter<ROSParticipantInfo>>,
  node_writer: NoKeyDataWriter<
    'a,
    ROSParticipantInfo,
    CDR_serializer_adapter<ROSParticipantInfo, LittleEndian>,
  >,
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
      .create_datareader_no_key(None, dtopic, ROSDiscoveryTopic::get_qos())?;

    let node_writer = ros_context
      .get_ros_discovery_publisher()
      .create_datawriter_no_key(None, dtopic, ROSDiscoveryTopic::get_qos())?;

    Ok(RosParticipant {
      nodes: HashMap::new(),
      external_nodes: HashMap::new(),
      node_reader,
      node_writer,
      ros_context,
    })
  }

  pub fn get_ros_participant_info(&self) -> ROSParticipantInfo {
    ROSParticipantInfo::new(
      Gid::from_guid(self.ros_context.domain_participant.get_guid()),
      self.nodes.iter().map(|(_, p)| p.clone()).collect(),
    )
  }

  pub fn add_node_info(&mut self, mut node_info: NodeInfo) {
    node_info
      .reader_guid
      .push(Gid::from_guid(self.node_reader.get_guid()));

    node_info
      .writer_guid
      .push(Gid::from_guid(self.node_writer.get_guid()));

    match self.nodes.insert(node_info.get_full_name(), node_info) {
      Some(_) => (),
      None => self.write_info(),
    }
  }

  pub fn remove_node_info(&mut self, node_info: &NodeInfo) {
    match self.nodes.remove(&node_info.get_full_name()) {
      Some(_) => self.write_info(),
      None => (),
    }
  }

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

pub struct RosContext {
  domain_participant: DomainParticipant,
  ros_discovery_topic: Option<Topic>,
  ros_discovery_publisher: Publisher,
  ros_discovery_subscriber: Subscriber,
  ros_parameter_events_topic: Topic,
  ros_rosout_topic: Topic,
}

impl RosContext {
  pub fn new(
    domain_participant: DomainParticipant,
    is_ros_participant_thread: bool,
  ) -> Result<RosContext, Error> {
    let ros_discovery_topic = if is_ros_participant_thread {
      Some(domain_participant.create_topic(
        &ROSDiscoveryTopic::topic_name().to_string(),
        TypeDesc::new(ROSDiscoveryTopic::type_name().to_string()),
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
      TypeDesc::new(ParameterEventsTopic::type_name().to_string()),
      &ParameterEventsTopic::get_qos(),
    )?;

    let ros_rosout_topic = domain_participant.create_topic(
      RosOutTopic::topic_name(),
      TypeDesc::new(RosOutTopic::type_name().to_string()),
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

pub struct NodeOptions {
  domain_id: u16,
  enable_rosout: bool,
}

impl NodeOptions {
  pub fn new(domain_id: u16, enable_rosout: bool) -> NodeOptions {
    NodeOptions {
      domain_id,
      enable_rosout,
    }
  }
}

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

  pub fn name(mut self, name: &str) -> RosNodeBuilder<'a> {
    self.name = Some(name.to_string());
    self
  }

  pub fn namespace(mut self, namespace: &str) -> RosNodeBuilder<'a> {
    self.namespace = Some(namespace.to_string());
    self
  }

  pub fn node_options(mut self, node_options: NodeOptions) -> RosNodeBuilder<'a> {
    self.node_options = Some(node_options);
    self
  }

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
  rosout_writer: Option<NoKeyDataWriter<'a, Log, CDR_serializer_adapter<Log, LittleEndian>>>,
  rosout_reader: Option<NoKeyDataReader<'a, Log, CDR_deserializer_adapter<Log>>>,
  parameter_events_writer:
    NoKeyDataWriter<'a, ParameterEvents, CDR_serializer_adapter<ParameterEvents, LittleEndian>>,
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
          .create_datawriter_no_key(None, rosout_topic, RosOutTopic::get_qos())?,
      )
    } else {
      None
    };

    let parameter_events_writer = ros_context
      .get_ros_discovery_publisher()
      .create_datawriter_no_key(None, paramtopic, ParameterEventsTopic::get_qos())?;

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

    let node_info = NodeInfo {
      node_namespace: self.namespace.to_owned(),
      node_name: self.name.to_owned(),
      reader_guid,
      writer_guid,
    };

    node_info
  }

  pub fn add_reader(&mut self, reader: GUID) {
    self.readers.insert(reader);
  }

  pub fn add_writer(&mut self, writer: GUID) {
    self.writers.insert(writer);
  }

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
    let topic =
      domain_participant.create_topic(&oname, TypeDesc::new(String::from(type_name)), &qos)?;
    Ok(topic)
  }

  fn create_ros_nokey_subscriber<D: DeserializeOwned + 'static, DA: DeserializerAdapter<D> + 'a>(
    &mut self,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<Box<dyn IDataReader<D, DA> + 'a>, Error> {
    let qos = match qos {
      Some(qos) => qos,
      None => QosPolicies::qos_none(),
    };

    match self
      .ros_context
      .get_ros_discovery_subscriber()
      .create_datareader_no_key::<D, DA>(None, topic, qos)
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
    let qos = match qos {
      Some(qos) => qos,
      None => QosPolicies::qos_none(),
    };

    match self
      .ros_context
      .get_ros_discovery_subscriber()
      .create_datareader::<D, DA>(None, topic, None, qos)
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
    let qos = match qos {
      Some(qos) => qos,
      None => QosPolicies::qos_none(),
    };

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
    let qos = match qos {
      Some(qos) => qos,
      None => QosPolicies::qos_none(),
    };

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
