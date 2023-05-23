use serde::{Deserialize, Serialize};

// TODO: PermissionsToken: section 8.4.2.1 of the Security specification (v.
// 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionsToken {}

impl PermissionsToken {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}

// TODO: PermissionsCredentialToken: section 8.4.2.2 of the Security
// specification (v. 1.1)
pub struct PermissionsCredentialToken {}

impl PermissionsCredentialToken {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}

// TODO: PermissionsHandle: section 8.4.2.3 of the Security specification (v.
// 1.1)
pub struct PermissionsHandle {}

impl PermissionsHandle {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}

// TODO: ParticipantSecurityAttributes: section 8.4.2.4 of the Security
// specification (v. 1.1)
pub struct ParticipantSecurityAttributes {}

impl ParticipantSecurityAttributes {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}

// TODO: TopicSecurityAttributes: section 8.4.2.6 of the Security specification
// (v. 1.1)
pub struct TopicSecurityAttributes {}

impl TopicSecurityAttributes {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}

// TODO: EndpointSecurityAttributes: section 8.4.2.7 of the Security
// specification (v. 1.1)
pub struct EndpointSecurityAttributes {}

impl EndpointSecurityAttributes {
  // Mock value used for development
  pub const MOCK: Self = Self {};
}
