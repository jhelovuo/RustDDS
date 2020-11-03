//! DDS interface
//!
//! # Examples
//!
//! ```
//! use atosdds::dds::DomainParticipant;
//! use atosdds::dds::qos::QosPolicyBuilder;
//! use atosdds::dds::qos::policy::Reliability;
//! use atosdds::dds::data_types::DDSDuration;
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
//! let some_topic = domain_participant.create_topic("some_topic", "SomeType", &qos).unwrap();
//!
//! // Used type needs Serialize for writers and Deserialize for readers
//! #[derive(Serialize, Deserialize)]
//! struct SomeType {
//!   a: i32
//! }
//!
//! // Creating DataReader requires type and deserializer adapter (which is recommended to be CDR).
//! let reader = subscriber
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
//! ```

pub(crate) mod datareader;
pub(crate) mod datasample;
mod datasample_cache;
pub(crate) mod datawriter;
pub(crate) mod ddsdata;
mod dp_event_wrapper;
pub(crate) mod interfaces;
mod message_receiver;
pub(crate) mod no_key;
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
  pub use super::datareader::SelectByKey;
  #[doc(inline)]
  pub use crate::structure::time::Time as DDSTime;
  #[doc(inline)]
  pub use crate::structure::time::Timestamp as DDSTimestamp;
  pub use crate::structure::guid::*;
  // TODO: move typedesc module somewhere better
  pub use crate::dds::typedesc::TypeDesc;
  pub use crate::dds::datasample::SampleInfo;
}

/// DDS Error
pub mod error {
  pub use super::values::result::*;
}

pub use participant::DomainParticipant;
pub use topic::Topic;
pub use pubsub::Subscriber;
pub use pubsub::Publisher;
#[doc(inline)]
pub use datawriter::DataWriter as KeyedDataWriter;
pub use no_key::datawriter::DataWriter;
#[doc(inline)]
pub use datareader::DataReader as KeyedDataReader;
pub use no_key::datareader::DataReader;

pub use interfaces::{
  IDataReader, IKeyedDataReader, IDataWriter, IKeyedDataWriter, IDataSample, IKeyedDataSample,
};
