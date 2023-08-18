// Builtin types for the access control plugin

use enumflags2::{bitflags, BitFlags};
use speedy::{Readable, Writable};

use crate::{
  security::{
    access_control::{PermissionsCredentialToken, PermissionsToken},
    authentication::authentication_builtin::types::CertificateAlgorithm,
    DataHolder, PluginEndpointSecurityAttributesMask, PluginParticipantSecurityAttributesMask,
    PluginSecurityAttributesMask, Property, SecurityError,
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

const PERMISSIONS_TOKEN_CLASS_ID: &str = "DDS:Access:Permissions:1.0";
const PERMISSIONS_TOKEN_SUBJECT_NAME_PROPERTY_NAME: &str = "dds.perm_ca.sn";
const PERMISSIONS_TOKEN_ALGORITHM_PROPERTY_NAME: &str = "dds.perm_ca.algo";
// 9.4.2.2
pub(super) struct BuiltinPermissionsToken {
  pub permissions_ca_subject_name: Option<String>,
  pub permissions_ca_algorithm: Option<CertificateAlgorithm>,
}
impl From<BuiltinPermissionsToken> for PermissionsToken {
  fn from(
    BuiltinPermissionsToken {
      permissions_ca_subject_name,
      permissions_ca_algorithm,
    }: BuiltinPermissionsToken,
  ) -> Self {
    PermissionsToken {
      data_holder: DataHolder {
        class_id: PERMISSIONS_TOKEN_CLASS_ID.into(),
        properties: [
          permissions_ca_subject_name.map(|subject_name| Property {
            name: PERMISSIONS_TOKEN_SUBJECT_NAME_PROPERTY_NAME.into(),
            value: subject_name,
            propagate: true,
          }),
          permissions_ca_algorithm.map(|algorithm| Property {
            name: PERMISSIONS_TOKEN_ALGORITHM_PROPERTY_NAME.into(),
            value: algorithm.into(),
            propagate: true,
          }),
        ]
        .into_iter()
        .collect::<Option<Vec<Property>>>()
        .unwrap_or_default(),
        binary_properties: Vec::new(),
      },
    }
  }
}

const PERMISSIONS_CREDENTIAL_TOKEN_CLASS_ID: &str = "DDS:Access:PermissionsCredential";
const PERMISSIONS_CREDENTIAL_TOKEN_CERTIFICATE_NAME: &str = "dds.perm.cert";
// 9.4.2.1
pub(super) struct BuiltinPermissionsCredentialToken {
  pub permissions_certificate: String, /* TODO: Should this be the whole permissions document
                                        * XML as a string? */
}
impl From<BuiltinPermissionsCredentialToken> for PermissionsCredentialToken {
  fn from(
    BuiltinPermissionsCredentialToken {
      permissions_certificate,
    }: BuiltinPermissionsCredentialToken,
  ) -> Self {
    PermissionsCredentialToken {
      data_holder: DataHolder {
        class_id: PERMISSIONS_CREDENTIAL_TOKEN_CLASS_ID.into(),
        properties: vec![Property {
          name: PERMISSIONS_CREDENTIAL_TOKEN_CERTIFICATE_NAME.into(),
          value: permissions_certificate,
          propagate: true,
        }],
        binary_properties: Vec::new(),
      },
    }
  }
}
