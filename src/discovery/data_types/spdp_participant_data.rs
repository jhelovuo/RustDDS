use std::collections::HashMap;

use mio::Token;
use serde::{de::Error, Deserialize, Serialize};
use chrono::Utc;
use bytes::Bytes;
use cdr_encoding_size::CdrEncodingSize;

use crate::{
  dds::{
    participant::DomainParticipant,
    qos::QosPolicies,
    rtps_reader_proxy::RtpsReaderProxy,
    rtps_writer_proxy::RtpsWriterProxy,
    traits::key::{Key, Keyed},
  },
  messages::{
    protocol_version::ProtocolVersion,
    submessages::submessage_elements::serialized_payload::RepresentationIdentifier,
    vendor_id::VendorId,
  },
  network::constant::*,
  serialization::{
    builtin_data_deserializer::BuiltinDataDeserializer,
    builtin_data_serializer::BuiltinDataSerializer, error::Result, pl_cdr_deserializer::*,
    pl_cdr_serializer::*,
  },
  structure::{
    builtin_endpoint::{BuiltinEndpointQos, BuiltinEndpointSet},
    duration::Duration,
    entity::RTPSEntity,
    guid::{EntityId, GUID},
    locator::Locator,
  },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpdpDiscoveredParticipantData {
  pub updated_time: chrono::DateTime<Utc>,
  pub protocol_version: ProtocolVersion,
  pub vendor_id: VendorId,
  pub expects_inline_qos: bool,
  pub participant_guid: GUID,
  pub metatraffic_unicast_locators: Vec<Locator>,
  pub metatraffic_multicast_locators: Vec<Locator>,
  pub default_unicast_locators: Vec<Locator>,
  pub default_multicast_locators: Vec<Locator>,
  pub available_builtin_endpoints: BuiltinEndpointSet,
  pub lease_duration: Option<Duration>,
  pub manual_liveliness_count: i32,
  pub builtin_endpoint_qos: Option<BuiltinEndpointQos>,
  pub entity_name: Option<String>,
}

impl SpdpDiscoveredParticipantData {
  pub(crate) fn as_reader_proxy(
    &self,
    is_metatraffic: bool,
    entity_id: Option<EntityId>,
  ) -> RtpsReaderProxy {
    let remote_reader_guid = GUID::new_with_prefix_and_id(
      self.participant_guid.prefix,
      match entity_id {
        Some(id) => id,
        None => EntityId::SPDP_BUILTIN_PARTICIPANT_READER,
      },
    );

    let mut proxy = RtpsReaderProxy::new(
      remote_reader_guid,
      QosPolicies::qos_none(), // TODO: What is the correct QoS value here?
    );
    proxy.expects_in_line_qos = self.expects_inline_qos;

    if !is_metatraffic {
      proxy.multicast_locator_list = self.default_multicast_locators.clone();
      proxy.unicast_locator_list = self.default_unicast_locators.clone();
    } else {
      proxy.multicast_locator_list = self.metatraffic_multicast_locators.clone();
      proxy.unicast_locator_list = self.metatraffic_unicast_locators.clone();
    }

    proxy
  }

  pub(crate) fn as_writer_proxy(
    &self,
    is_metatraffic: bool,
    entity_id: Option<EntityId>,
  ) -> RtpsWriterProxy {
    let remote_writer_guid = GUID::new_with_prefix_and_id(
      self.participant_guid.prefix,
      match entity_id {
        Some(id) => id,
        None => EntityId::SPDP_BUILTIN_PARTICIPANT_WRITER,
      },
    );

    let mut proxy = RtpsWriterProxy::new(
      remote_writer_guid,
      Vec::new(),
      Vec::new(),
      EntityId::UNKNOWN,
    );

    if is_metatraffic {
      // TODO: possible multicast addresses
      proxy.unicast_locator_list = self.metatraffic_unicast_locators.clone();
    } else {
      // TODO: possible multicast addresses
      proxy.unicast_locator_list = self.default_unicast_locators.clone();
    }

    proxy
  }

  pub fn from_local_participant(
    participant: &DomainParticipant,
    self_locators: &HashMap<Token, Vec<Locator>>,
    lease_duration: Duration,
  ) -> SpdpDiscoveredParticipantData {
    let metatraffic_multicast_locators = self_locators
      .get(&DISCOVERY_MUL_LISTENER_TOKEN)
      .cloned()
      .unwrap_or_default();

    let metatraffic_unicast_locators = self_locators
      .get(&DISCOVERY_LISTENER_TOKEN)
      .cloned()
      .unwrap_or_default();

    let default_multicast_locators = self_locators
      .get(&USER_TRAFFIC_MUL_LISTENER_TOKEN)
      .cloned()
      .unwrap_or_default();

    let default_unicast_locators = self_locators
      .get(&USER_TRAFFIC_LISTENER_TOKEN)
      .cloned()
      .unwrap_or_default();

    let builtin_endpoints = BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PARTICIPANT_ANNOUNCER
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR
      | BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_MESSAGE_DATA_WRITER
      | BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_MESSAGE_DATA_READER
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_TOPICS_ANNOUNCER
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_TOPICS_DETECTOR;

    SpdpDiscoveredParticipantData {
      updated_time: Utc::now(),
      protocol_version: ProtocolVersion::PROTOCOLVERSION_2_3,
      vendor_id: VendorId::THIS_IMPLEMENTATION,
      expects_inline_qos: false,
      participant_guid: participant.guid(),
      metatraffic_unicast_locators,
      metatraffic_multicast_locators,
      default_unicast_locators,
      default_multicast_locators,
      available_builtin_endpoints: BuiltinEndpointSet::from_u32(builtin_endpoints),
      lease_duration: Some(lease_duration),
      manual_liveliness_count: 0,
      builtin_endpoint_qos: None,
      entity_name: None,
    }
  }
}

impl PlCdrDeserialize for SpdpDiscoveredParticipantData {
  fn from_pl_cdr_bytes(
    input_bytes: &[u8],
    encoding: RepresentationIdentifier,
  ) -> Result<SpdpDiscoveredParticipantData> {
    BuiltinDataDeserializer::new()
      .parse_data(input_bytes, encoding)
      .generate_spdp_participant_data()
      .map_err(|e| {
        Error::custom(format!(
          "SpdpDiscoveredParticipantData::deserialize - {:?} - data was {:?}",
          e, &input_bytes,
        ))
      })
  }
}

impl PlCdrSerialize for SpdpDiscoveredParticipantData {
  fn to_pl_cdr_bytes(&self, encoding: RepresentationIdentifier) -> Result<Bytes> {
    BuiltinDataSerializer::from_participant_data(self).serialize_pl_cdr_to_Bytes(encoding)
  }
}

// We need a wrapper to distinguish between Participant and Endpoint GUIDs.
#[allow(non_camel_case_types)]
#[derive(
  PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Serialize, Deserialize, CdrEncodingSize, Hash,
)]
pub struct Participant_GUID(pub GUID);

impl Key for Participant_GUID {}

impl Keyed for SpdpDiscoveredParticipantData {
  type K = Participant_GUID;
  fn key(&self) -> Self::K {
    Participant_GUID(self.participant_guid)
  }
}

impl PlCdrDeserialize for Participant_GUID {
  fn from_pl_cdr_bytes(
    input_bytes: &[u8],
    encoding: RepresentationIdentifier,
  ) -> Result<Participant_GUID> {
    BuiltinDataDeserializer::new()
      .parse_data(input_bytes, encoding)
      .generate_participant_guid()
      .map_err(|e| {
        Error::custom(format!(
          "deserialize Participant_GUID - {:?} - data was {:?}",
          e, &input_bytes,
        ))
      })
  }
}

impl PlCdrSerialize for Participant_GUID {
  fn to_pl_cdr_bytes(&self, encoding: RepresentationIdentifier) -> Result<Bytes> {
    BuiltinDataSerializer::from_participant_guid(*self).serialize_pl_cdr_to_Bytes(encoding)
  }
}

#[cfg(test)]
mod tests {
  use byteorder::LittleEndian;

  use super::*;
  use crate::{
    dds::traits::serde_adapters::no_key::DeserializerAdapter,
    messages::submessages::{
      submessage_elements::serialized_payload::RepresentationIdentifier,
      submessages::EntitySubmessage,
    },
    serialization::{
      cdr_serializer::to_bytes, message::Message, pl_cdr_deserializer::PlCdrDeserializerAdapter,
      pl_cdr_serializer::PlCdrSerialize, submessage::*,
    },
    test::test_data::*,
  };

  #[test]
  fn pdata_deserialize_serialize() {
    let data = spdp_participant_data_raw();

    let rtpsmsg = Message::read_from_buffer(&data).unwrap();
    let submsgs = rtpsmsg.submessages();

    for submsg in &submsgs {
      match &submsg.body {
        SubmessageBody::Entity(v) => match v {
          EntitySubmessage::Data(d, _) => {
            let participant_data: SpdpDiscoveredParticipantData =
              PlCdrDeserializerAdapter::from_bytes(
                &d.serialized_payload.as_ref().unwrap().value,
                RepresentationIdentifier::PL_CDR_LE,
              )
              .unwrap();
            let sdata = participant_data
              .to_pl_cdr_bytes(RepresentationIdentifier::PL_CDR_LE)
              .unwrap();
            eprintln!("message data = {:?}", &data);
            eprintln!(
              "payload    = {:?}",
              &d.serialized_payload.as_ref().unwrap().value.to_vec()
            );
            eprintln!("deserialized  = {:?}", &participant_data);
            eprintln!("serialized = {:?}", &sdata);
            // order cannot be known at this point
            //assert_eq!(
            //  sdata.len(),
            //  d.serialized_payload.as_ref().unwrap().value.len()
            //);

            let mut participant_data_2: SpdpDiscoveredParticipantData =
              PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE)
                .unwrap();
            // force timestamps to be the same, as these are not serialized/deserialized,
            // but stamped during deserialization
            participant_data_2.updated_time = participant_data.updated_time;

            eprintln!("again deserialized = {:?}", &participant_data_2);
            let _sdata_2 =
              to_bytes::<SpdpDiscoveredParticipantData, LittleEndian>(&participant_data_2).unwrap();
            // now the order of bytes should be the same
            assert_eq!(&participant_data_2, &participant_data);
          }

          _ => continue,
        },
        SubmessageBody::Interpreter(_) => (),
      }
    }
  }
}
