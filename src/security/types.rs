use speedy::{Readable};
use enumflags2::{bitflags};

// Property_t type from section 7.2.1 of the Security specification (v. 1.1)
pub struct Property {
  name: String,
  value: String,
  propagate: bool,
}

// BinaryProperty_t type from section 7.2.2 of the Security specification (v.
// 1.1)
pub struct BinaryProperty {
  name: String,
  value: Vec<u8>,
  propagate: bool, // propagate field is not serialized
}

// DataHolder type from section 7.2.3 of the Security specification (v. 1.1)
pub struct DataHolder {
  class_id: String,
  properties: Vec<Property>,
  binary_properties: Vec<BinaryProperty>,
}

// Token type from section 7.2.4 of the Security specification (v. 1.1)
pub type Token = DataHolder;

// ParticipantBuiltinTopicDataSecure from section 7.4.1.6 of the Security
// specification
pub struct ParticipantBuiltinTopicDataSecure {}

// PublicationBuiltinTopicDataSecure from section 7.4.1.7 of the Security
// specification
pub struct PublicationBuiltinTopicDataSecure {}

// SubscriptionBuiltinTopicDataSecure from section 7.4.1.8 of the Security
// specification
pub struct SubscriptionBuiltinTopicDataSecure {}

// Result type with generic OK type. Error type is SecurityError.
pub type SecurityResult<T> = std::result::Result<T, SecurityError>;

// Something like the SecurityException of the specification
#[derive(Debug, thiserror::Error)]
#[error("Security exception: {msg}")]
pub struct SecurityError {
  msg: String,
}

// DDS Security spec v1.1 Section 7.2.8 EndpointSecurityInfo
// This is communicated over Discovery

#[derive(Debug, Readable, Clone, PartialEq, Eq, )]
pub struct EndpointSecurityInfo {
  endpoint_security_attributes: EndpointSecurityAttributesMask,
  plugin_endpoint_security_attributes: PluginEndpointSecurityAttributesMask,
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[bitflags]
#[repr(u32)]
pub enum EndpointSecurityAttributesMask {
  IsValid = 0x8000_0000, // (0x1 << 31)
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy)]
#[bitflags]
#[repr(u32)]
pub enum PluginEndpointSecurityAttributesMask {
  IsValid = 0x8000_0000, // (0x1 << 31)
}
