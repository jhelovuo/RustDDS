pub mod ack_nack;
pub mod data;
pub mod data_frag;
pub mod gap;
pub mod heartbeat;
pub mod heartbeat_frag;
pub mod nack_frag;

#[cfg(feature = "security")]
pub mod secure_body;

pub mod info_destination;
pub mod info_reply;
pub mod info_source;
pub mod info_timestamp;

pub mod elements;

#[cfg(feature = "security")]
pub mod secure_postfix;
#[cfg(feature = "security")]
pub mod secure_prefix;
#[cfg(feature = "security")]
pub mod secure_rtps_postfix;
#[cfg(feature = "security")]
pub mod secure_rtps_prefix;

pub mod submessage;

#[allow(non_camel_case_types)]
pub mod submessage_flag;
pub mod submessage_header;
pub mod submessage_kind;

#[allow(clippy::module_inception)]
pub mod submessages {
  pub use super::{
    ack_nack::*, data::*, data_frag::*, elements::RepresentationIdentifier, gap::*, heartbeat::*,
    heartbeat_frag::*, info_destination::*, info_reply::*, info_timestamp::*, nack_frag::*,
    submessage::*, submessage_flag::*, submessage_header::*, submessage_kind::*,
  };
}
