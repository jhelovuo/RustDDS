use speedy::Readable;
use enumflags2::bitflags;
use serde::{Deserialize, Serialize};

use crate::structure::parameter_id::ParameterId;

// Property_t type from section 7.2.1 of the Security specification (v. 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
  name: String,
  value: String,
  propagate: bool,
}

// BinaryProperty_t type from section 7.2.2 of the Security specification (v.
// 1.1)
pub struct BinaryProperty {
  pub(crate) name: String, // public because of serialization
  value: Vec<u8>,
  propagate: bool, // propagate field is not serialized
}

// DataHolder type from section 7.2.3 of the Security specification (v. 1.1)
// fields need to be public to make (de)serializable
pub struct DataHolder {
  pub(crate) class_id: String,
  pub(crate) properties: Vec<Property>,
  pub(crate) binary_properties: Vec<BinaryProperty>,
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
  pub(crate) msg: String, 
}


// DDS Security spec v1.1 Section 7.2.7 ParticipantSecurityInfo
// This is communicated over Discovery

#[derive(Debug, Readable, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantSecurityInfo {
  participant_security_attributes: ParticipantSecurityAttributesMask,
  plugin_participant_security_attributes: PluginParticipantSecurityAttributesMask,
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy, Serialize, Deserialize)]
#[bitflags]
#[repr(u32)]
#[allow(clippy::enum_variant_names)]
// Clippy complains, because all variant names have the same prefix "Is",
// but we blame the DDS Security spec for naming.
pub enum ParticipantSecurityAttributesMask {
  IsValid = 0x8000_0000, // (0x1 << 31) -- only this bit is understood ouside security plugins

  // DDS Security specification v1.1
  // Section 8.4.2.5 Definition of the ParticipantSecurityAttributesMask
  // Table 28
  IsRTPSProtected = 0b000_0001,
  IsDiscoveryProtected = 0b000_0010,
  IsLivelinessProtected = 0b000_0100,
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy, Serialize, Deserialize)]
#[bitflags]
#[repr(u32)]
#[allow(clippy::enum_variant_names)]
// Clippy complains, because all variant names have the same prefix.
pub enum PluginParticipantSecurityAttributesMask {
  IsValid = 0x8000_0000, // (0x1 << 31)

  // DDS Security specification v1.1
  // Section 9.4.2.4 Definition of the PluginParticipantSecurityAttributesMask
  // Table 60
  IsRTPSEncrypted = 0b0000_0001,
  IsDiscoveryEncrypted = 0b0000_0010,
  IsLivelinessEncrypted = 0b0000_0100,
  IsRTPSOriginAuthetincated = 0b0000_1000,
  IsDiscoveryOriginAuthenticated = 0b0001_0000,
  IsLivelinessOriginAuthenticated = 0b0010_0000,
}


// serialization helper struct
#[derive(Serialize, Deserialize)]
pub(crate) struct ParticipantSecurityInfoData {
  parameter_id: ParameterId,
  parameter_length: u16,
  security_info: ParticipantSecurityInfo,
}

impl ParticipantSecurityInfoData {
  pub fn new(security_info: ParticipantSecurityInfo) -> Self {
    Self {
      parameter_id: ParameterId::PID_PARTICIPANT_SECURITY_INFO,
      parameter_length: 8, // 2x u32
      security_info,
    }
  }
}



// DDS Security spec v1.1 Section 7.2.8 EndpointSecurityInfo
// This is communicated over Discovery

#[derive(Debug, Readable, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointSecurityInfo {
  endpoint_security_attributes: EndpointSecurityAttributesMask,
  plugin_endpoint_security_attributes: PluginEndpointSecurityAttributesMask,
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy, Serialize, Deserialize)]
#[bitflags]
#[repr(u32)]
#[allow(clippy::enum_variant_names)]
// Clippy complains, because all variant names have the same prefix "Is",
// but we blame the DDS Security spec for naming.
pub enum EndpointSecurityAttributesMask {
  IsValid = 0x8000_0000, // (0x1 << 31) -- only this bit is understood ouside security plugins

  // DDS Security specification v1.1
  // Section 8.4.2.8 Definition of the EndpointSecurityAttributesMask
  // Table 31
  IsReadProtected = 0b0000_0001,
  IsWriteProtected = 0b0000_0010,
  IsDiscoveryProtected = 0b0000_0100,
  IsSubmessageProtected = 0b0000_1000,
  IsPayloadProtected = 0b0001_0000,
  IsKeyProtected = 0b0010_0000,
  IsLivelinessProtected = 0b0100_0000,
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Clone, Copy, Serialize, Deserialize)]
#[bitflags]
#[repr(u32)]
#[allow(clippy::enum_variant_names)]
// Clippy complains, because all variant names have the same prefix.
pub enum PluginEndpointSecurityAttributesMask {
  IsValid = 0x8000_0000, // (0x1 << 31)

  // DDS Security specification v1.1
  // Section 9.4.2.6 Definition of the PluginEndpointSecurityAttributesMask
  // Table 62
  IsSubmessageEncrypted = 0b0000_0001,
  IsPayloadEncrypted = 0b0000_0010,
  IsSubmessageOriginAuthenticated = 0b0000_0100,
}

// serialization helper struct
#[derive(Serialize, Deserialize)]
pub(crate) struct EndpointSecurityInfoData {
  parameter_id: ParameterId,
  parameter_length: u16,
  security_info: EndpointSecurityInfo,
}

impl EndpointSecurityInfoData {
  pub fn new(security_info: EndpointSecurityInfo) -> Self {
    Self {
      parameter_id: ParameterId::PID_ENDPOINT_SECURITY_INFO,
      parameter_length: 8, // 2x u32
      security_info,
    }
  }
}
