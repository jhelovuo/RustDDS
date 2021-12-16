//! DDS interface - Most commonly needed items should be re-exported directly to
//! crate top level and modules [`no_key`](crate::no_key) and
//! [`with_key`](crate::with_key).

mod datasample_cache;
pub(crate) mod ddsdata;
mod dp_event_loop;
mod fragment_assembler;
mod helpers;
mod message_receiver;
pub mod sampleinfo;

/// Participating in NoKey topics.
pub mod no_key;
/// Participating in WithKey topics.
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
pub(crate) mod values;
pub(crate) mod writer;

// Public interface

/// DDS Quality of Service
pub mod qos;

pub mod statusevents;

/// Datatypes needed for overall operability with this crate

#[deprecated(
  since = "0.7.0",
  note = "Please use re-exports directly from crate top level instead."
)]
#[doc(hidden)]
pub mod data_types {
  pub use crate::{
    dds::sampleinfo::SampleInfo,
    discovery::data_types::topic_data::{DiscoveredTopicData, SubscriptionBuiltinTopicData},
    structure::guid::*,
  };
  pub use super::{
    readcondition::ReadCondition,
    topic::{Topic, TopicKind},
    traits::key::BuiltInTopicKey,
  };
  #[doc(inline)]
  pub use super::with_key::datareader::SelectByKey;
  // TODO: move typedesc module somewhere better
  pub use crate::dds::typedesc::TypeDesc;
}

// DDS Error and Result types
pub use crate::dds::values::result::*;

pub use participant::DomainParticipant;
pub use topic::{Topic, TopicKind};
pub use pubsub::{Publisher, Subscriber};

// Discovery results
pub use crate::discovery::data_types::topic_data::{DiscoveredTopicData, SubscriptionBuiltinTopicData};

