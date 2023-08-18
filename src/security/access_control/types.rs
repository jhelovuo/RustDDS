use enumflags2::BitFlags;
use speedy::{Readable, Writable};

use crate::security::{
  DataHolder, EndpointSecurityAttributesMask, EndpointSecurityAttributesMaskFlags,
  ParticipantSecurityAttributesMask, ParticipantSecurityAttributesMaskFlags,
  PluginParticipantSecurityAttributesMask, PluginSecurityAttributesMask, Property,
};

// PermissionsToken: section 8.4.2.1 of the Security specification (v.
// 1.1)
#[derive(Debug, Clone, PartialEq, Eq, Readable, Writable)]
pub struct PermissionsToken {
  // TODO: Readable & Writable are now derived, but likely need to be implemented manually.
  // Readable and Writable are needed to (de)serialize to(from) ParameterList.
  // Note: The implementation has to observe CDR alignment rules.
  // Automatic derive does not do so.
  pub data_holder: DataHolder,
}

impl From<DataHolder> for PermissionsToken {
  fn from(value: DataHolder) -> Self {
    Self { data_holder: value }
  }
}

// PermissionsCredentialToken: section 8.4.2.2 of the Security
// specification (v. 1.1)
pub struct PermissionsCredentialToken {
  pub data_holder: DataHolder,
}

impl From<DataHolder> for PermissionsCredentialToken {
  fn from(value: DataHolder) -> Self {
    Self { data_holder: value }
  }
}

// PermissionsHandle: section 8.4.2.3 of the Security specification (v.
// 1.1)
pub type PermissionsHandle = u32;

// ParticipantSecurityAttributes: section 8.4.2.4 of the Security
// specification (v. 1.1)
#[derive(Clone)]
pub struct ParticipantSecurityAttributes {
  pub allow_unauthenticated_participants: bool,
  pub is_access_protected: bool,
  pub is_rtps_protected: bool,
  pub is_discovery_protected: bool,
  pub is_liveliness_protected: bool,
  pub plugin_participant_attributes: PluginParticipantSecurityAttributesMask,
  pub ac_participant_properties: Vec<Property>,
}
impl ParticipantSecurityAttributes {
  pub fn empty() -> Self {
    Self {
      allow_unauthenticated_participants: false,
      is_access_protected: false,
      is_rtps_protected: false,
      is_discovery_protected: false,
      is_liveliness_protected: false,
      plugin_participant_attributes: PluginSecurityAttributesMask::empty(),
      ac_participant_properties: Vec::new(),
    }
  }
}

impl From<ParticipantSecurityAttributes> for ParticipantSecurityAttributesMask {
  fn from(
    ParticipantSecurityAttributes {
      is_rtps_protected,
      is_discovery_protected,
      is_liveliness_protected,
      ..
    }: ParticipantSecurityAttributes,
  ) -> Self {
    let mut mask = BitFlags::from_flag(ParticipantSecurityAttributesMaskFlags::IsValid);
    if is_rtps_protected {
      mask.insert(ParticipantSecurityAttributesMaskFlags::IsRTPSProtected);
    }
    if is_discovery_protected {
      mask.insert(ParticipantSecurityAttributesMaskFlags::IsDiscoveryProtected);
    }
    if is_liveliness_protected {
      mask.insert(ParticipantSecurityAttributesMaskFlags::IsLivelinessProtected);
    }
    Self(mask)
  }
}

// TODO: TopicSecurityAttributes: section 8.4.2.6 of the Security specification
// (v. 1.1)
#[derive(Clone, Copy)]
pub struct TopicSecurityAttributes {
  pub is_read_protected: bool,
  pub is_write_protected: bool,
  pub is_discovery_protected: bool,
  pub is_liveliness_protected: bool,
}
impl TopicSecurityAttributes {
  pub fn empty() -> Self {
    Self {
      is_read_protected: false,
      is_write_protected: false,
      is_discovery_protected: false,
      is_liveliness_protected: false,
    }
  }
}

// TODO: EndpointSecurityAttributes: section 8.4.2.7 of the Security
// specification (v. 1.1)
#[derive(Clone)]
pub struct EndpointSecurityAttributes {
  pub topic_security_attributes: TopicSecurityAttributes,
  pub is_submessage_protected: bool,
  pub is_payload_protected: bool,
  pub is_key_protected: bool,
  pub plugin_endpoint_attributes: PluginParticipantSecurityAttributesMask,
  pub ac_endpoint_properties: Vec<Property>,
}
impl EndpointSecurityAttributes {
  pub fn empty() -> Self {
    Self {
      topic_security_attributes: TopicSecurityAttributes::empty(),
      is_submessage_protected: false,
      is_payload_protected: false,
      is_key_protected: false,
      plugin_endpoint_attributes: PluginSecurityAttributesMask(0),
      ac_endpoint_properties: Vec::new(),
    }
  }
}

impl From<EndpointSecurityAttributes> for EndpointSecurityAttributesMask {
  fn from(
    EndpointSecurityAttributes {
      topic_security_attributes:
        TopicSecurityAttributes {
          is_read_protected,
          is_write_protected,
          is_discovery_protected,
          is_liveliness_protected,
        },
      is_submessage_protected,
      is_payload_protected,
      is_key_protected,
      ..
    }: EndpointSecurityAttributes,
  ) -> Self {
    let mut mask = BitFlags::from_flag(EndpointSecurityAttributesMaskFlags::IsValid);
    if is_read_protected {
      mask.insert(EndpointSecurityAttributesMaskFlags::IsReadProtected);
    }
    if is_write_protected {
      mask.insert(EndpointSecurityAttributesMaskFlags::IsWriteProtected);
    }
    if is_discovery_protected {
      mask.insert(EndpointSecurityAttributesMaskFlags::IsDiscoveryProtected);
    }
    if is_submessage_protected {
      mask.insert(EndpointSecurityAttributesMaskFlags::IsSubmessageProtected);
    }
    if is_payload_protected {
      mask.insert(EndpointSecurityAttributesMaskFlags::IsPayloadProtected);
    }
    if is_key_protected {
      mask.insert(EndpointSecurityAttributesMaskFlags::IsKeyProtected);
    }
    if is_liveliness_protected {
      mask.insert(EndpointSecurityAttributesMaskFlags::IsLivelinessProtected);
    }
    Self(mask)
  }
}
