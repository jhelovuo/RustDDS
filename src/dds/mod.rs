//! DDS interface
//!
//! # DDS usage summary
//!
//! * Crate a `DomaniParticipant`. You have to choose a domain id. The default value is zero.
//! * Create or find a `Topic` from the `DomainParticipant`. Topics have a name and a type.
//! * Create a `Publisher` and/or `Subscriber` from the `DomainParticipant`.
//! * To receive data, create a `DataReader` from `Subscriber` and `Topic`.
//! * To send data, create a `DataWriter`from `Publisher` and `Topic`.
//! * Data from `DataReader` can be read or taken. Taking removes the data samples from the DataReader,
//!   whereas reading only marks them as read.
//! * Topics are either WITH_KEY or NO_KEY. WITH_KEY topics are like map data structures, containing multiple
//!   instances (map items), identified by key. The key must be something that can be extracted from the
//!   data samples. Instances can be created (published) and deleted (disposed).
//!   NO_KEY topics have always only one instance of the data.
//! * Data is sent and received in consecutive samples. When read, a smaple is accompanied with metadata (SampleInfo).
//!
//! # Interfacing Rust data types to DDS
//! * DDS takes care of serialization and deserialization.
//! In order to do this, the payload data must be Serde serializable/deserializable.
//! * If your data is to be communicated over a WITH_KEY topic, the payload data type must
//!   implement `Keyed` trait from this crate.
//!
//! # Examples
//!
//! ```
//! use atosdds::dds::DomainParticipant;
//! use atosdds::dds::{No_Key_DataReader as DataReader, No_Key_DataWriter as DataWriter, no_key::datasample::DataSample};
//! use atosdds::dds::qos::QosPolicyBuilder;
//! use atosdds::dds::qos::policy::Reliability;
//! use atosdds::dds::data_types::DDSDuration;
//! use atosdds::dds::data_types::TopicKind;
//! use atosdds::serialization::{CDRSerializerAdapter, CDRDeserializerAdapter};
//! use serde::{Serialize, Deserialize};
//!
//! // DomainParticipant is always necessary
//! let domain_participant = DomainParticipant::new(0);
//!
//! let qos = QosPolicyBuilder::new()
//!   .reliability(Reliability::Reliable { max_blocking_time: DDSDuration::DURATION_ZERO })
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
//! let some_topic = domain_participant.create_topic("some_topic", "SomeType", &qos, TopicKind::NO_KEY).unwrap();
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
//!     None,
//!     None)
//!   .unwrap();
//!
//! // Creating DataWriter required type and serializer adapter (which is recommended to be CDR).
//! let writer = publisher
//!   .create_datawriter_no_key::<SomeType, CDRSerializerAdapter<SomeType>>(
//!     None,
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

mod datasample_cache;
pub(crate) mod ddsdata;
mod dp_event_wrapper;
mod message_receiver;
mod sampleinfo;

/// Participating in NO_KEY topics.
pub mod no_key;
/// Participating in WITH_KEY topics.
pub mod with_key;

pub(crate) mod participant;
pub(crate) mod pubsub;
pub(crate) mod readcondition;
pub(crate) mod reader;
pub(crate) mod rtps_reader_proxy;
pub(crate) mod rtps_writer_proxy;
pub(crate) mod topic;
pub mod traits;
pub(crate) mod typedesc;
pub(crate) mod util;
pub(crate) mod values;
pub(crate) mod writer;

// Public interface

/// DDS Quality of Service
pub mod qos;

/// Datatypes needed for overall operability with this crate
pub mod data_types {
  pub use crate::discovery::data_types::topic_data::{
    DiscoveredTopicData, SubscriptionBuiltinTopicData,
  };
  #[doc(inline)]
  pub use crate::structure::duration::Duration as DDSDuration;
  pub use super::readcondition::ReadCondition;
  pub use super::with_key::datareader::SelectByKey;
  #[doc(inline)]
  pub use crate::structure::time::Timestamp as DDSTimestamp;
  pub use crate::structure::guid::*;
  // TODO: move typedesc module somewhere better
  pub use crate::dds::typedesc::TypeDesc;
  pub use crate::dds::sampleinfo::SampleInfo;
  pub use crate::structure::topic_kind::TopicKind; // AKA dds::topic::TopicKind
  pub use crate::structure::entity::Entity;
}

/// DDS Error
pub mod error {
  pub use super::values::result::*;
}

pub use participant::DomainParticipant;
pub use topic::Topic;
pub use pubsub::Subscriber;
pub use pubsub::Publisher;

/// DDS DataWriter for with_key topics.
#[doc(inline)]
pub use with_key::datawriter::DataWriter as With_Key_DataWriter;

/// DDS DataWriter for no_key topics.
#[doc(inline)]
pub use no_key::datawriter::DataWriter as No_Key_DataWriter;

/// DDS DataReader for with_key topics.
#[doc(inline)]
pub use with_key::datareader::DataReader as With_Key_DataReader;

/// DDS DataReader for no_key topics.
#[doc(inline)]
pub use no_key::datareader::DataReader as No_Key_DataReader;
