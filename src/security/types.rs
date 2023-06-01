use bytes::Bytes;
use enumflags2::bitflags;
use speedy::{Context, Readable, Reader, Writable, Writer};
use serde::{Serialize, Deserialize};

use crate::serialization::speedy_pl_cdr_helpers::*;
use crate::{discovery, security, dds::qos, serialization, Keyed, GUID};
use crate::RepresentationIdentifier;
use crate::serialization::pl_cdr_adapters::{PlCdrSerialize, PlCdrDeserialize};


// Property_t type from section 7.2.1 of the Security specification (v. 1.1)
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)] // for CDR in Discovery
pub struct Property {
  name: String,
  value: String,
  propagate: bool, // NOT SERIALIZED
}

impl<'a, C: Context> Readable<'a, C> for Property {
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let name: StringWithNul = reader.read_value()?;

    read_pad(reader, name.len(), 4)?; // pad according to previous read
    let value: StringWithNul = reader.read_value()?;

    Ok(Property {
      name: name.into(),
      value: value.into(),
      propagate: true, // since we read this from thw wire, it was propagated
    })
  }
}

// Writing several strings is a bit complicated, because
// we have to keep track of alignment.
// Again, alignment comes BEFORE string length, or vector item count, not after
// string.
impl<C: Context> Writable<C> for Property {
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    let name = StringWithNul::from(self.name.clone());
    // nothing yet to pad
    writer.write_value(&name)?;

    write_pad(writer, name.len(), 4)?;
    let value = StringWithNul::from(self.value.clone());
    writer.write_value(&value)?;

    Ok(())
  }
}

impl Property {
  pub fn serialized_len(&self) -> usize {
    let first = 4 + self.name.len() + 1;
    let misalign = first % 4;
    let align = if misalign > 0 { 4 - misalign } else { 0 };
    let second = 4 + self.value.len() + 1;
    first + align + second
  }
}

// BinaryProperty_t type from section 7.2.2 of the Security specification (v.
// 1.1)
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)] // for CDR in Discovery
#[serde(into = "repr::BinaryProperty", from = "repr::BinaryProperty")]
pub struct BinaryProperty {
  pub(crate) name: String, // public because of serialization
  pub(crate) value: Bytes, // Serde cannot derive for Bytes, therefore use repr::
  pub(crate) propagate: bool, // propagate field is not serialized
}

mod repr {
  use serde::{Serialize, Deserialize};

  #[derive(Serialize, Deserialize)]
  pub struct BinaryProperty {
    pub(crate) name: String, 
    pub(crate) value: Vec<u8>,
    pub(crate) propagate: bool, 
  }

  impl From<BinaryProperty> for super::BinaryProperty {
    fn from(bp: BinaryProperty) -> super::BinaryProperty {
      super::BinaryProperty {
        name: bp.name,
        value: bp.value.into(),
        propagate: bp.propagate,
      }
    }
  }

  impl From<super::BinaryProperty> for BinaryProperty {
    fn from(bp: super::BinaryProperty) -> BinaryProperty {
      BinaryProperty {
        name: bp.name,
        value: bp.value.into(),
        propagate: bp.propagate,
      }      
    }
  }
}

impl BinaryProperty {
  pub fn serialized_len(&self) -> usize {
    let first = 4 + self.name.len() + 1;
    let misalign = first % 4;
    let align = if misalign > 0 { 4 - misalign } else { 0 };
    let second = 4 + self.value.len(); // no nul terminator byte here
    first + align + second
  }
}

impl<'a, C: Context> Readable<'a, C> for BinaryProperty {
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let name: StringWithNul = reader.read_value()?;

    read_pad(reader, name.len(), 4)?; // pad according to previous read
    let value: Vec<u8> = reader.read_value()?;

    Ok(BinaryProperty {
      name: name.into(),
      value: value.into(),
      propagate: true, // since we read this from thw wire, it was propagated
    })
  }
}

// Writing several strings is a bit complicated, because
// we have to keep track of alignment.
// Again, alignment comes BEFORE string length, or vector item count, not after
// string.
impl<C: Context> Writable<C> for BinaryProperty {
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    let name = StringWithNul::from(self.name.clone());
    writer.write_value(&name)?;

    write_pad(writer, name.len(), 4)?;
    writer.write_value(&<Vec<u8>>::from(self.value.clone()))?;

    Ok(())
  }
}


// Tag type from section 7.2.5 of the DDS Security specification (v. 1.1)
// The silly thing is almost the same as "Property"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
  pub(crate) name: String,
  pub(crate) value: String,
}

impl<'a, C: Context> Readable<'a, C> for Tag {
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let name: StringWithNul = reader.read_value()?;

    read_pad(reader, name.len(), 4)?; // pad according to previous read
    let value: StringWithNul = reader.read_value()?;

    Ok(Tag {
      name: name.into(),
      value: value.into(),
    })
  }
}

// See alignment comment in "Property"
impl<C: Context> Writable<C> for Tag {
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    let name = StringWithNul::from(self.name.clone());
    writer.write_value(&name)?;

    write_pad(writer, name.len(), 4)?;
    writer.write_value(&StringWithNul::from(self.value.clone()))?;

    Ok(())
  }
}

impl Tag {
  pub fn serialized_len(&self) -> usize {
    let first = 4 + self.name.len() + 1;
    let misalign = first % 4;
    let align = if misalign > 0 { 4 - misalign } else { 0 };
    let second = 4 + self.value.len() + 1;
    first + align + second
  }
}





// DataHolder type from section 7.2.3 of the Security specification (v. 1.1)
// fields need to be public to make (de)serializable
#[derive(Clone)]
#[derive(Serialize, Deserialize)] // for CDR in Discovery
pub struct DataHolder {
  pub(crate) class_id: String,
  pub(crate) properties: Vec<Property>,
  pub(crate) binary_properties: Vec<BinaryProperty>,
}

// Token type from section 7.2.4 of the Security specification (v. 1.1)
pub type Token = DataHolder;


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

#[derive(Debug, Clone, PartialEq, Eq, Readable, Writable)]
pub struct ParticipantSecurityInfo {
  participant_security_attributes: ParticipantSecurityAttributesMask,
  plugin_participant_security_attributes: PluginParticipantSecurityAttributesMask,
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Clone, Copy, Readable, Writable)]
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

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Clone, Copy, Readable, Writable)]
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

// DDS Security spec v1.1 Section 7.2.8 EndpointSecurityInfo
// This is communicated over Discovery

#[derive(Debug, Clone, PartialEq, Eq, Readable, Writable)]
pub struct EndpointSecurityInfo {
  endpoint_security_attributes: EndpointSecurityAttributesMask,
  plugin_endpoint_security_attributes: PluginEndpointSecurityAttributesMask,
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Clone, Copy, Readable, Writable)]
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

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Clone, Copy, Readable, Writable)]
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


// ParticipantBuiltinTopicDataSecure from section 7.4.1.6 of the Security
// specification
pub struct ParticipantBuiltinTopicDataSecure {
  pub participant_data: discovery::spdp_participant_data::SpdpDiscoveredParticipantData,
  pub identity_status_token: security::authentication::IdentityStatusToken,
}
impl Keyed for ParticipantBuiltinTopicDataSecure {
  type K = Participant_GUID;
  fn key(&self) -> Self::K {
    self.participant_data.key()
  }
}
impl PlCdrDeserialize for ParticipantBuiltinTopicDataSecure {
  fn from_pl_cdr_bytes(input_bytes: &[u8], encoding: RepresentationIdentifier) -> serialization::Result<Self> {
    todo!()
  }
}

impl PlCdrSerialize for ParticipantBuiltinTopicDataSecure {
  fn to_pl_cdr_bytes(&self, encoding: RepresentationIdentifier) -> serialization::Result<Bytes> {
    todo!()
  }
}

// PublicationBuiltinTopicDataSecure from section 7.4.1.7 of the Security
// specification
pub struct PublicationBuiltinTopicDataSecure {
  pub discovered_writer_data: discovery::sedp_messages::DiscoveredWriterData,
  pub data_tags: qos::policy::DataTag,
}

impl Keyed for PublicationBuiltinTopicDataSecure {
  type K = Endpoint_GUID;
  fn key(&self) -> Self::K {
    self.discovered_writer_data.key()
  }
}
impl PlCdrDeserialize for PublicationBuiltinTopicDataSecure {
  fn from_pl_cdr_bytes(input_bytes: &[u8], encoding: RepresentationIdentifier) -> serialization::Result<Self> {
    todo!()
  }
}

impl PlCdrSerialize for PublicationBuiltinTopicDataSecure {
  fn to_pl_cdr_bytes(&self, encoding: RepresentationIdentifier) -> serialization::Result<Bytes> {
    todo!()
  }
}


// SubscriptionBuiltinTopicDataSecure from section 7.4.1.8 of the Security
// specification
pub struct SubscriptionBuiltinTopicDataSecure {
  pub discovered_reader_data: discovery::sedp_messages::DiscoveredReaderData,
  pub data_tags: qos::policy::DataTag,
}
impl Keyed for SubscriptionBuiltinTopicDataSecure {
  type K = Endpoint_GUID;
  fn key(&self) -> Self::K {
    self.discovered_reader_data.key()
  }
}
impl PlCdrDeserialize for SubscriptionBuiltinTopicDataSecure {
  fn from_pl_cdr_bytes(input_bytes: &[u8], encoding: RepresentationIdentifier) -> serialization::Result<Self> {
    todo!()
  }
}

impl PlCdrSerialize for SubscriptionBuiltinTopicDataSecure {
  fn to_pl_cdr_bytes(&self, encoding: RepresentationIdentifier) -> serialization::Result<Bytes> {
    todo!()
  }
}

// ParticipantStatelessMessage from section 7.4.3.3 of the Security
// specification
#[derive(Serialize, Deserialize)]
pub struct ParticipantStatelessMessage {
  generic: ParticipantGenericMessage,
}
// The specification defines and uses the following specific values for the GenericMessageClassId:
// #define GMCLASSID_SECURITY_AUTH_REQUEST “dds.sec.auth_request”
// #define GMCLASSID_SECURITY_AUTH_HANDSHAKE “dds.sec.auth”

impl Keyed for ParticipantStatelessMessage {
  type K = GUID;

  fn key(&self) -> Self::K {
    self.generic.key()
  }
}



// ParticipantVolatileMessageSecure from section 7.4.4.3 of the Security
// specification
//
// spec: typedef ParticipantVolatileMessageSecure ParticipantGenericMessage;
#[derive(Serialize, Deserialize)]
pub struct ParticipantVolatileMessageSecure {
  generic: ParticipantGenericMessage,
}
// The specification defines and uses the following specific values for the GenericMessageClassId:
// #define GMCLASSID_SECURITY_PARTICIPANT_CRYPTO_TOKENS ”dds.sec.participant_crypto_tokens”
// #define GMCLASSID_SECURITY_DATAWRITER_CRYPTO_TOKENS ”dds.sec.datawriter_crypto_tokens”
// #define GMCLASSID_SECURITY_DATAREADER_CRYPTO_TOKENS ”dds.sec.datareader_crypto_tokens”

impl Keyed for ParticipantVolatileMessageSecure {
  type K = GUID;

  fn key(&self) -> Self::K {
    self.generic.key()
  }
}


use crate::structure::rpc;
use crate::discovery::spdp_participant_data::Participant_GUID;
use crate::discovery::sedp_messages::Endpoint_GUID;

// This is the transport (message) type for specialized versions above.
// DDS Security Spec v1.1
// Section 7.2.6 ParticipantGenericMessage
#[derive(Serialize, Deserialize)]
pub struct ParticipantGenericMessage {
  pub message_identity: rpc::SampleIdentity,
  pub related_message_identity: rpc::SampleIdentity,
  // GUIDs here need not be typed Endpoint_GUID or Participant_GUID,
  // because CDR serialization does not need the distiction, unlike PL_CDR.
  pub destination_participant_guid: GUID, //target for the request. Can be GUID_UNKNOWN 
  pub destination_endpoint_guid: GUID,
  pub source_endpoint_guid: GUID,
  pub message_class_id: String,
  pub message_data: Vec<DataHolder>,
}

impl Keyed for ParticipantGenericMessage {
  type K = GUID;

  fn key(&self) -> Self::K {
    self.source_endpoint_guid
  }
}
