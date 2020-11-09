//! ROS2 interface using DDS module
//!
//! # Examples
//!
//! ```
//! use atosdds::dds::DomainParticipant;
//! use atosdds::dds::data_types::TopicKind;
//! use atosdds::dds::data_types::Entity;
//! use atosdds::ros2::RosContext;
//! use atosdds::ros2::RosParticipant;
//! use atosdds::ros2::NodeOptions;
//! use atosdds::ros2::RosNode;
//! use atosdds::ros2::IRosNodeControl;
//! use atosdds::ros2::RosNodeBuilder;
//! use atosdds::ros2::builtin_datatypes::NodeInfo;
//! use atosdds::dds::qos::QosPolicies;
//! use atosdds::serialization::CDRSerializerAdapter;
//!
//!
//! // DomainParticipant is always needed
//! let domain_participant = DomainParticipant::new(0);
//!
//! // RosContext should be defined for each thread and second parameter set true if RosParticipant
//! // is handled in this thread
//! let ros_context = RosContext::new(domain_participant.clone(), true).unwrap();
//!
//! // RosParticipant is needed for defined RosNodes to be visible in ROS2 network.
//! let mut ros_participant = RosParticipant::new(&ros_context).unwrap();
//!
//! // Node options simply adjust configuration of the node (as in ROS Client library (RCL)).
//! let ros_node_options = NodeOptions::new(domain_participant.domain_id(), false);
//!
//! // Creating some topic for RosNode
//! let some_topic = RosNode::create_ros_topic(
//!     &domain_participant,
//!     "some_topic_name",
//!     "NodeInfo",
//!     QosPolicies::qos_none(),
//!     TopicKind::NO_KEY)
//!   .unwrap();
//!
//! // Topic has to live longer that node or readers/writers
//! {
//!   // declaring ros node using builder
//!   let mut ros_node = RosNodeBuilder::new()
//!     .name("some_node_name")
//!     .namespace("/some_namespace")
//!     .node_options(ros_node_options)
//!     .ros_context(&ros_context)
//!     .build()
//!     .unwrap();
//!
//!   // declaring some writer that use non keyed types
//!   let some_writer = ros_node
//!     .create_ros_nokey_publisher::<NodeInfo, CDRSerializerAdapter<_>>(
//!       &some_topic, None)
//!     .unwrap();
//!
//!   // RosNode needs to be updated manually about our writers and readers (lifetime magic)
//!   ros_node.add_writer(some_writer.get_guid());
//!
//!   // Readers and RosParticipant implement mio Evented trait and thus function the same way as
//!   // std::sync::mpcs and can be handled the same way for reading the data
//!
//!   // When you add or remove nodes remember to update RosParticipant so others in the ROS2
//!   // network can find your nodes
//!   ros_participant.add_node_info(ros_node.generate_node_info());
//! }
//! ```

/// Some builtin datatypes needed for ROS2 communication
pub mod builtin_datatypes;
/// Some convenience topic infos for ROS2 communication
pub mod builtin_topics;

pub(crate) mod ros_node;

pub use ros_node::*;

pub type RosSubscriber<'a, D, DA> = crate::dds::no_key::datareader::DataReader<'a, D, DA>;

pub type KeyedRosSubscriber<'a, D, DA> = crate::dds::with_key::datareader::DataReader<'a, D, DA>;

pub type RosPublisher<'a, D, SA> = crate::dds::no_key::datawriter::DataWriter<'a, D, SA>;

pub type KeyedRosPublisher<'a, D, SA> = crate::dds::with_key::datawriter::DataWriter<'a, D, SA>;
