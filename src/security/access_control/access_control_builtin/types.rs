// Builtin types for the access control plugin

use enumflags2::{bitflags, BitFlags};
use speedy::{Readable, Writable};

use crate::{
  security::{
    PluginEndpointSecurityAttributesMask, PluginParticipantSecurityAttributesMask,
    PluginSecurityAttributesMask, SecurityError,
  },
  security_error,
};

// 9.4.2.3
// This is also used in the builtin cryptographic plugin, hence pub(in
// crate::security)
pub(in crate::security) struct BuiltinPluginParticipantSecurityAttributes {
  pub is_rtps_encrypted: bool,
  pub is_discovery_encrypted: bool,
  pub is_liveliness_encrypted: bool,
  pub is_rtps_origin_authenticated: bool,
  pub is_discovery_origin_authenticated: bool,
  pub is_liveliness_origin_authenticated: bool,
}
impl TryFrom<PluginParticipantSecurityAttributesMask>
  for BuiltinPluginParticipantSecurityAttributes
{
  type Error = SecurityError;
  fn try_from(
    PluginSecurityAttributesMask(value): PluginParticipantSecurityAttributesMask,
  ) -> Result<Self, Self::Error> {
    BitFlags::<BuiltinPluginParticipantSecurityAttributesMaskFlags>::try_from(value)
      .map_err(|e| {
        security_error!(
          "Could not convert to BuiltinPluginParticipantSecurityAttributesMask: {e:?}"
        )
      })
      .and_then(|mask| {
        if mask.contains(BuiltinPluginParticipantSecurityAttributesMaskFlags::IsValid) {
          Ok(Self {
            is_rtps_encrypted: mask
              .contains(BuiltinPluginParticipantSecurityAttributesMaskFlags::IsRTPSEncrypted),
            is_discovery_encrypted: mask
              .contains(BuiltinPluginParticipantSecurityAttributesMaskFlags::IsDiscoveryEncrypted),
            is_liveliness_encrypted: mask
              .contains(BuiltinPluginParticipantSecurityAttributesMaskFlags::IsLivelinessEncrypted),
            is_rtps_origin_authenticated: mask.contains(
              BuiltinPluginParticipantSecurityAttributesMaskFlags::IsRTPSOriginAuthenticated,
            ),
            is_discovery_origin_authenticated: mask.contains(
              BuiltinPluginParticipantSecurityAttributesMaskFlags::IsDiscoveryOriginAuthenticated,
            ),
            is_liveliness_origin_authenticated: mask.contains(
              BuiltinPluginParticipantSecurityAttributesMaskFlags::IsLivelinessOriginAuthenticated,
            ),
          })
        } else {
          Err(security_error!(
            "The IsValid flag of BuiltinPluginParticipantSecurityAttributesMask was set to false."
          ))
        }
      })
  }
}
impl From<BuiltinPluginParticipantSecurityAttributes> for PluginParticipantSecurityAttributesMask {
  fn from(
    BuiltinPluginParticipantSecurityAttributes {
      is_rtps_encrypted,
      is_discovery_encrypted,
      is_liveliness_encrypted,
      is_rtps_origin_authenticated,
      is_discovery_origin_authenticated,
      is_liveliness_origin_authenticated,
    }: BuiltinPluginParticipantSecurityAttributes,
  ) -> Self {
    let mut mask =
      BitFlags::from_flag(BuiltinPluginParticipantSecurityAttributesMaskFlags::IsValid);
    if is_rtps_encrypted {
      mask.insert(BuiltinPluginParticipantSecurityAttributesMaskFlags::IsRTPSEncrypted);
    }
    if is_discovery_encrypted {
      mask.insert(BuiltinPluginParticipantSecurityAttributesMaskFlags::IsDiscoveryEncrypted);
    }
    if is_liveliness_encrypted {
      mask.insert(BuiltinPluginParticipantSecurityAttributesMaskFlags::IsLivelinessEncrypted);
    }
    if is_rtps_origin_authenticated {
      mask.insert(BuiltinPluginParticipantSecurityAttributesMaskFlags::IsRTPSOriginAuthenticated);
    }
    if is_discovery_origin_authenticated {
      mask.insert(
        BuiltinPluginParticipantSecurityAttributesMaskFlags::IsDiscoveryOriginAuthenticated,
      );
    }
    if is_liveliness_origin_authenticated {
      mask.insert(
        BuiltinPluginParticipantSecurityAttributesMaskFlags::IsLivelinessOriginAuthenticated,
      );
    }
    Self(mask.bits())
  }
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Clone, Copy, Readable, Writable)]
#[bitflags]
#[repr(u32)]
#[allow(clippy::enum_variant_names)]
// Clippy complains, because all variant names have the same prefix.
pub(super) enum BuiltinPluginParticipantSecurityAttributesMaskFlags {
  IsValid = 0x8000_0000, // (0x1 << 31)

  // DDS Security specification v1.1
  // Section 9.4.2.4 Definition of the PluginParticipantSecurityAttributesMask
  // Table 60
  IsRTPSEncrypted = 0b0000_0001,
  IsDiscoveryEncrypted = 0b0000_0010,
  IsLivelinessEncrypted = 0b0000_0100,
  IsRTPSOriginAuthenticated = 0b0000_1000,
  IsDiscoveryOriginAuthenticated = 0b0001_0000,
  IsLivelinessOriginAuthenticated = 0b0010_0000,
}

// 9.4.2.5
// This is also used in the builtin cryptographic plugin, hence pub(in
// crate::security)
pub(in crate::security) struct BuiltinPluginEndpointSecurityAttributes {
  pub is_submessage_encrypted: bool,
  pub is_submessage_origin_authenticated: bool,
  pub is_payload_encrypted: bool,
}
impl TryFrom<PluginEndpointSecurityAttributesMask> for BuiltinPluginEndpointSecurityAttributes {
  type Error = SecurityError;
  fn try_from(
    PluginSecurityAttributesMask(value): PluginEndpointSecurityAttributesMask,
  ) -> Result<Self, Self::Error> {
    BitFlags::<BuiltinPluginEndpointSecurityAttributesMaskFlags>::try_from(value)
      .map_err(|e| {
        security_error!("Could not convert to BuiltinPluginEndpointSecurityAttributesMask: {e:?}")
      })
      .and_then(|mask| {
        if mask.contains(BuiltinPluginEndpointSecurityAttributesMaskFlags::IsValid) {
          Ok(Self {
            is_submessage_encrypted: mask
              .contains(BuiltinPluginEndpointSecurityAttributesMaskFlags::IsSubmessageEncrypted),
            is_submessage_origin_authenticated: mask.contains(
              BuiltinPluginEndpointSecurityAttributesMaskFlags::IsSubmessageOriginAuthenticated,
            ),
            is_payload_encrypted: mask
              .contains(BuiltinPluginEndpointSecurityAttributesMaskFlags::IsPayloadEncrypted),
          })
        } else {
          Err(security_error!(
            "The IsValid flag of BuiltinPluginEndpointSecurityAttributesMask was set to false."
          ))
        }
      })
  }
}
impl From<BuiltinPluginEndpointSecurityAttributes> for PluginEndpointSecurityAttributesMask {
  fn from(
    BuiltinPluginEndpointSecurityAttributes {
      is_submessage_encrypted,
      is_submessage_origin_authenticated,
      is_payload_encrypted,
    }: BuiltinPluginEndpointSecurityAttributes,
  ) -> Self {
    let mut mask = BitFlags::from_flag(BuiltinPluginEndpointSecurityAttributesMaskFlags::IsValid);
    if is_submessage_encrypted {
      mask.insert(BuiltinPluginEndpointSecurityAttributesMaskFlags::IsSubmessageEncrypted);
    }
    if is_submessage_origin_authenticated {
      mask
        .insert(BuiltinPluginEndpointSecurityAttributesMaskFlags::IsSubmessageOriginAuthenticated);
    }
    if is_payload_encrypted {
      mask.insert(BuiltinPluginEndpointSecurityAttributesMaskFlags::IsPayloadEncrypted);
    }
    Self(mask.bits())
  }
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Clone, Copy, Readable, Writable)]
#[bitflags]
#[repr(u32)]
#[allow(clippy::enum_variant_names)]
// Clippy complains, because all variant names have the same prefix.
pub(super) enum BuiltinPluginEndpointSecurityAttributesMaskFlags {
  IsValid = 0x8000_0000, // (0x1 << 31)

  // DDS Security specification v1.1
  // Section 9.4.2.6 Definition of the PluginEndpointSecurityAttributesMask
  // Table 62
  IsSubmessageEncrypted = 0b0000_0001,
  IsPayloadEncrypted = 0b0000_0010,
  IsSubmessageOriginAuthenticated = 0b0000_0100,
}
