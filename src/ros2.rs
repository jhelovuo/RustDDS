//! ROS2 interface using DDS module - DO NOT USE - Use [ros2-client](https://crates.io/crates/ros2-client) instead.

/// Some builtin datatypes needed for ROS2 communication
pub mod builtin_datatypes;
/// Some convenience topic infos for ROS2 communication
pub mod builtin_topics;

pub(crate) mod ros_node;

pub use ros_node::*;

pub type RosSubscriber<D, DA> = crate::dds::no_key::datareader::DataReader<D, DA>;

pub type KeyedRosSubscriber<D, DA> = crate::dds::with_key::datareader::DataReader<D, DA>;

pub type RosPublisher<D, SA> = crate::dds::no_key::datawriter::DataWriter<D, SA>;

pub type KeyedRosPublisher<D, SA> = crate::dds::with_key::datawriter::DataWriter<D, SA>;

// Short-hand notation for CDR serialization

pub type RosSubscriberCdr<D> =
  crate::dds::no_key::datareader::DataReader<D, crate::serialization::CDRDeserializerAdapter<D>>;

pub type KeyedRosSubscriberCdr<D> =
  crate::dds::with_key::datareader::DataReader<D, crate::serialization::CDRDeserializerAdapter<D>>;

pub type RosPublisherCdr<D> =
  crate::dds::no_key::datawriter::DataWriter<D, crate::serialization::CDRSerializerAdapter<D>>;

pub type KeyedRosPublisherCdr<D> =
  crate::dds::with_key::datawriter::DataWriter<D, crate::serialization::CDRSerializerAdapter<D>>;
