use std::collections::HashMap;

use crate::dds::{
  participant::DomainParticipant, pubsub::Publisher, pubsub::Subscriber, topic::Topic,
  typedesc::TypeDesc, values::result::Error,
};

use super::builtin_topics::ROSDiscoveryTopic;

pub trait IRosNode {
  fn get_name(&self) -> &str;
  fn get_namespace(&self) -> &str;
  fn get_fully_qualified_name(&self) -> String;
  fn get_options(&self) -> &NodeOptions;
  fn get_domain_id(&self) -> u16;
}

pub trait IRosNodeControl {
  // fn create_ros_publisher<D>() -> DataWriter<D>;
}

pub struct NodeOptions {
  domain_id: u16,
  enable_rosout: bool,
}

impl NodeOptions {}

pub struct RosContext {
  domain_participant: DomainParticipant,
  ros_nodes: HashMap<String, RosNode>,
}

impl RosContext {
  pub fn new(domain_id: Option<u16>) -> RosContext {
    let domain_participant = match domain_id {
      Some(id) => DomainParticipant::new(id),
      None => DomainParticipant::new(0),
    };
    RosContext {
      domain_participant,
      ros_nodes: HashMap::new(),
    }
  }

  pub fn create_ros_node(
    &mut self,
    name: &str,
    namespace: &str,
    options: NodeOptions,
  ) -> Result<&RosNode, Error> {
    let mut options = options;
    options.domain_id = self.domain_participant.domain_id();
    let ros_node = RosNode::new(&self.domain_participant, name, namespace, options)?;
    let node_name = String::from(name);
    self.ros_nodes.insert(node_name.clone(), ros_node);
    Ok(self.ros_nodes.get(&node_name).unwrap())
  }
}

pub struct RosNode {
  // node info
  name: String,
  namespace: String,
  options: NodeOptions,

  // node control
  node_topic: Topic,
  node_publisher: Publisher,
  node_subscriber: Subscriber,
}

impl RosNode {
  pub fn new(
    domain_participant: &DomainParticipant,
    name: &str,
    namespace: &str,
    options: NodeOptions,
  ) -> Result<RosNode, Error> {
    let node_topic = domain_participant.create_topic(
      &ROSDiscoveryTopic::topic_name(),
      TypeDesc::new(ROSDiscoveryTopic::type_name()),
      &ROSDiscoveryTopic::get_qos(),
    )?;

    let node_publisher = domain_participant.create_publisher(&ROSDiscoveryTopic::get_qos())?;
    let node_subscriber = domain_participant.create_subscriber(&ROSDiscoveryTopic::get_qos())?;

    Ok(RosNode {
      name: String::from(name),
      namespace: String::from(namespace),
      options,
      node_topic,
      node_publisher,
      node_subscriber,
    })
  }
}

impl IRosNode for RosNode {
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
