use serde::{Serialize, Deserialize};
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuiltinEndpointSet {
  value: u32,
}

impl BuiltinEndpointSet {
  pub const DISC_BUILTIN_ENDPOINT_PARTICIPANT_ANNOUNCER: u32 = 0x00000001;
  pub const DISC_BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR: u32 = 0x000000002;
  pub const DISC_BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER: u32 = 0x00000004;
  pub const DISC_BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR: u32 = 0x00000008;
  pub const DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER: u32 = 0x00000010;
  pub const DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR: u32 = 0x00000020;

  pub const DISC_BUILTIN_ENDPOINT_TOPICS_ANNOUNCER: u32 = 0x08000000;
  pub const DISC_BUILTIN_ENDPOINT_TOPICS_DETECTOR: u32 = 0x10000000;
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuiltinEndpointQos {
  value: u32,
}

impl BuiltinEndpointQos {
  pub const BEST_EFFORT_PARTICIPANT_MESSAGE_DATA_READER: u32 = 0x00000001;
}
