//! A pure Rust implementation of Data Distribution Service (DDS).
//!
//! DDS is an object-oriented API [specified](https://www.omg.org/spec/DDSI-RTPS/2.3) by
//! the Object Management Group.
//!
//! DDS communicates over the network using the [RTPS](https://www.omg.org/spec/DDSI-RTPS/2.3)
//! protocol, which by default runs over UDP/IP.
//!
//! This implementation does not attempt to make an accurate implementation of
//! the DDS object API, as it would be quite unnatural to use in Rust as such.
//! However, we aim for functional compatibility, while at the same time using
//! Rust techniques and conventions.
//!
//! Additionally, there is a [ROS2](https://index.ros.org/doc/ros2/) interface, that is simpler to use than DDS
//! when communicating to ROS2 components.
//!
//! # DDS usage summary
//!
//! * Create a [`DomainParticipant`]. You have to choose a domain id. The
//!   default value is zero.
//! * Create or find a [`Topic`] from the [`DomainParticipant`]. Topics have a
//!   name and a type.
//! * Create a [`Publisher`] and/or [`Subscriber`] from the
//!   [`DomainParticipant`].
//! * To receive data, create a [`DataReader`](with_key::DataReader) from
//!   [`Subscriber`] and [`Topic`].
//! * To send data, create a [`DataWriter`](with_key::DataWriter) from
//!   [`Publisher`] and [`Topic`].
//! * Data from `DataReader` can be read or taken. Taking removes the data
//!   samples from the DataReader, whereas reading only marks them as read.
//! * Topics are either WithKey or NoKey. WithKey topics are like map data
//!   structures, containing multiple instances (map items), identified by key.
//!   The key must be something that can be extracted from the data samples.
//!   Instances can be created (published) and deleted (disposed). NoKey topics
//!   have always only one instance of the data.
//! * Data is sent and received in consecutive samples. When read, a smaple is
//!   accompanied with metadata (SampleInfo).
//!
//! # Interfacing Rust data types to DDS
//!
//! * DDS takes care of serialization and deserialization.
//! In order to do this, the payload data must be Serde
//! serializable/deserializable.
//! * If your data is to be communicated over a WithKey topic, the payload data
//!   type must implement [`Keyed`] trait from this crate.
//! * If you are using CDR serialization (DDS default), then use [`CDRSerializerAdapter`](../serialization/CdrSerializerAdapter) and [`CdrDeserializerAdapter`](../serialization/CdrDeserializerAdapter)
//!   when such adapters are required. If you need to use another serialization format, then you should find or write
//!   a [Serde data format](https://serde.rs/data-format.html) implementation and wrap it as a (De)SerializerAdaper.
//!
//! # Usage Example
//!
//! ```
//! use rustdds::*;
//! use rustdds::no_key::{DataReader, DataWriter, DataSample}; // We use a NO_KEY topic here
//! use serde::{Serialize, Deserialize};
//!
//! // DomainParticipant is always necessary
//! let domain_participant = DomainParticipant::new(0).unwrap();
//!
//! let qos = QosPolicyBuilder::new()
//!   .reliability(Reliability::Reliable { max_blocking_time: rustdds::Duration::DURATION_ZERO })
//!   .build();
//!
//! // DDS Subscriber, only one is necessary for each thread (slight difference to
//! // DDS specification)
//! let subscriber = domain_participant.create_subscriber(&qos).unwrap();
//!
//! // DDS Publisher, only one is necessary for each thread (slight difference to
//! // DDS specification)
//! let publisher = domain_participant.create_publisher(&qos).unwrap();
//!
//! // Some DDS Topic that we can write and read from (basically only binds readers
//! // and writers together)
//! let some_topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
//!
//! // Used type needs Serialize for writers and Deserialize for readers
//! #[derive(Serialize, Deserialize)]
//! struct SomeType {
//!   a: i32
//! }
//!
//! // Creating DataReader requires type and deserializer adapter (which is recommended to be CDR).
//! // Reader needs to be mutable if any operations are used.
//! let mut reader = subscriber
//!   .create_datareader_no_key::<SomeType, CDRDeserializerAdapter<SomeType>>(
//!     &some_topic,
//!     None)
//!   .unwrap();
//!
//! // Creating DataWriter required type and serializer adapter (which is recommended to be CDR).
//! let writer = publisher
//!   .create_datawriter_no_key::<SomeType, CDRSerializerAdapter<SomeType>>(
//!     &some_topic,
//!     None)
//!   .unwrap();
//!
//! // Readers implement mio Evented trait and thus function the same way as
//! // std::sync::mpcs and can be handled the same way for reading the data
//!
//! let some_data = SomeType { a: 1 };
//!
//! // This should send the data to all who listen "some_topic" topic.
//! writer.write(some_data, None).unwrap();
//!
//! // ... Some data has arrived at some point for the reader
//! let data_sample = if let Ok(Some(value)) = reader.read_next_sample() {
//!   value
//! } else {
//!   // no data has arrived
//!   return;
//! };
//!
//! // Getting reference to actual data from the data sample
//! let actual_data = data_sample.value();
//! ```
#![deny(clippy::all)]
#![warn(clippy::needless_pass_by_value, clippy::semicolon_if_nothing_returned)]
#![allow(
  clippy::option_map_unit_fn,
  // option_map_unit_fn suggests changing option.map( ) with () return value to if let -construct,
  // but that may break code flow.
  dead_code,
)]

#[macro_use]
mod serialization_test;
#[macro_use]
mod checked_impl;
mod discovery;
mod messages;
mod network;
pub(crate) mod structure;

#[cfg(test)]
mod test;

// Public modules
pub mod dds;
pub mod ros2;
/// Helpers for (De)serialization and definitions of (De)serializer adapters
pub mod serialization;

// Re-exports from crate root to simplify usage

pub use dds::{
  participant::DomainParticipant,
  pubsub::{Publisher, Subscriber},
  qos,
  qos::{policy::*, QosPolicies, QosPolicyBuilder},
  readcondition::ReadCondition,
  sampleinfo::{InstanceState, NotAliveGenerationCounts, SampleInfo, SampleState, ViewState},
  statusevents::StatusEvented,
  topic::{Topic, TopicDescription, TopicKind},
  traits::{Key, Keyed},
  typedesc::TypeDesc,
  values::result::{Error, Result},
};

/// Components used to access NO_KEY Topics
pub mod no_key {
  pub use crate::dds::no_key::*;
  pub use crate::dds::traits::serde_adapters::no_key::*;
}

/// Components used to access WITH_KEY Topics
pub mod with_key {
  pub use crate::dds::with_key::*;
  pub use crate::dds::traits::serde_adapters::with_key::*;
}

pub use serialization::{CdrSerializer, CdrDeserializer, CDRSerializerAdapter, CDRDeserializerAdapter};
pub use structure::{duration::Duration, time::Timestamp};
