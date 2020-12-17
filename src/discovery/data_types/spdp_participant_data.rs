use serde::{Serialize, Deserialize};

use crate::{
  dds::{
    traits::{key::Keyed},
    rtps_reader_proxy::RtpsReaderProxy,
    participant::DomainParticipant,
    rtps_writer_proxy::RtpsWriterProxy,
  },
  network::util::get_local_multicast_locators,
  network::util::get_local_unicast_socket_address,
};

use crate::messages::{protocol_version::ProtocolVersion, vendor_id::VendorId};
use crate::{
  structure::{
    locator::LocatorList,
    guid::{EntityId, GUID},
    duration::Duration,
    builtin_endpoint::{BuiltinEndpointSet, BuiltinEndpointQos},
    entity::RTPSEntity,
  },
};

use crate::{
  serialization::{
    builtin_data_serializer::BuiltinDataSerializer,
    builtin_data_deserializer::BuiltinDataDeserializer,
  },
  network::constant::*,
};

//use std::{time::Duration as StdDuration};

use chrono::Utc;

#[derive(Debug, Clone)]
pub struct SPDPDiscoveredParticipantData {
  pub updated_time: u64,
  pub protocol_version: Option<ProtocolVersion>,
  pub vendor_id: Option<VendorId>,
  pub expects_inline_qos: Option<bool>,
  pub participant_guid: Option<GUID>,
  pub metatraffic_unicast_locators: LocatorList,
  pub metatraffic_multicast_locators: LocatorList,
  pub default_unicast_locators: LocatorList,
  pub default_multicast_locators: LocatorList,
  pub available_builtin_endpoints: Option<BuiltinEndpointSet>,
  pub lease_duration: Option<Duration>,
  pub manual_liveliness_count: Option<i32>,
  pub builtin_enpoint_qos: Option<BuiltinEndpointQos>,
  pub entity_name: Option<String>,
}

impl SPDPDiscoveredParticipantData {
  pub(crate) fn as_reader_proxy(
    &self,
    is_metatraffic: bool,
    entity_id: Option<EntityId>,
  ) -> RtpsReaderProxy {
    let remote_reader_guid = GUID::new_with_prefix_and_id(
      self.participant_guid.unwrap().guidPrefix,
      match entity_id {
        Some(id) => id,
        None => EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
      },
    );

    let mut proxy = RtpsReaderProxy::new(remote_reader_guid);
    proxy.expects_in_line_qos = match self.expects_inline_qos {
      Some(v) => v,
      None => false,
    };

    if !is_metatraffic {
      // TODO: possible multicast addresses
      // proxy.multicast_locator_list = self.default_multicast_locators.clone();
      proxy.unicast_locator_list = self.default_unicast_locators.clone();
    } else {
      // TODO: possible multicast addresses
      // proxy.multicast_locator_list = self.metatraffic_multicast_locators.clone();
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
      self.participant_guid.unwrap().guidPrefix,
      match entity_id {
        Some(id) => id,
        None => EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
      },
    );

    let mut proxy = RtpsWriterProxy::new(
      remote_writer_guid,
      Vec::new(),
      Vec::new(),
      EntityId::ENTITYID_UNKNOWN,
    );

    if !is_metatraffic {
      // TODO: possible multicast addresses
      proxy.unicast_locator_list = self.default_unicast_locators.clone();
    } else {
      // TODO: possible multicast addresses
      proxy.unicast_locator_list = self.metatraffic_unicast_locators.clone();
    }

    proxy
  }

  pub fn from_participant(
    participant: &DomainParticipant,
    lease_duration: Duration,
  ) -> SPDPDiscoveredParticipantData {
    let spdp_multicast_port = get_spdp_well_known_multicast_port(participant.domain_id());
    let metatraffic_multicast_locators = get_local_multicast_locators(spdp_multicast_port);

    let spdp_unicast_port =
      get_spdp_well_known_unicast_port(participant.domain_id(), participant.participant_id());
    let metatraffic_unicast_locators = get_local_unicast_socket_address(spdp_unicast_port);

    let multicast_port = get_user_traffic_multicast_port(participant.domain_id());
    let default_multicast_locators = get_local_multicast_locators(multicast_port);

    let unicast_port =
      get_user_traffic_unicast_port(participant.domain_id(), participant.participant_id());
    let default_unicast_locators = get_local_unicast_socket_address(unicast_port);

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

    SPDPDiscoveredParticipantData {
      updated_time: Utc::now().timestamp_nanos() as u64,
      protocol_version: Some(ProtocolVersion::PROTOCOLVERSION_2_3),
      vendor_id: Some(VendorId::THIS_IMPLEMENTATION),
      expects_inline_qos: Some(false),
      participant_guid: Some(participant.get_guid()),
      metatraffic_unicast_locators,
      metatraffic_multicast_locators,
      default_unicast_locators,
      default_multicast_locators,
      available_builtin_endpoints: Some(BuiltinEndpointSet::from_u32(builtin_endpoints)),
      lease_duration: Some(Duration::from(lease_duration)),
      manual_liveliness_count: None,
      builtin_enpoint_qos: None,
      entity_name: None,
    }
  }
}

impl Keyed for SPDPDiscoveredParticipantData {
  type K = GUID; // placeholder
  fn get_key(&self) -> Self::K {
    match self.participant_guid {
      Some(g) => g,
      None => GUID::GUID_UNKNOWN,
    }
  }
}

impl<'de> Deserialize<'de> for SPDPDiscoveredParticipantData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let visitor = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(visitor)?;
    Ok(res.generate_spdp_participant_data())
  }
}

impl Serialize for SPDPDiscoveredParticipantData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_participant_data(&self);
    builtin_data_serializer.serialize::<S>(serializer, true)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::messages::submessages::submessages::EntitySubmessage;
  use crate::serialization::message::Message;
  use crate::serialization::submessage::*;
  use crate::serialization::pl_cdr_deserializer::PlCdrDeserializerAdapter;
  use crate::serialization::cdr_serializer::{to_bytes};
  use byteorder::LittleEndian;
  use crate::{
    messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier,
    test::test_data::*,
  };
  use crate::dds::traits::serde_adapters::DeserializerAdapter;

  #[test]
  fn pdata_deserialize_serialize() {
    let data = spdp_participant_data_raw();

    let rtpsmsg = Message::read_from_buffer(&data).unwrap();
    let submsgs = rtpsmsg.submessages();

    for submsg in submsgs.iter() {
      match &submsg.body {
        SubmessageBody::Entity(v) => match v {
          EntitySubmessage::Data(d, _) => {
            let participant_data: SPDPDiscoveredParticipantData =
              PlCdrDeserializerAdapter::from_bytes(
                &d.serialized_payload.as_ref().unwrap().value,
                RepresentationIdentifier::PL_CDR_LE,
              )
              .unwrap();
            let sdata =
              to_bytes::<SPDPDiscoveredParticipantData, LittleEndian>(&participant_data).unwrap();
            // order cannot be known at this point
            assert_eq!(
              sdata.len(),
              d.serialized_payload.as_ref().unwrap().value.len()
            );

            let participant_data_2: SPDPDiscoveredParticipantData =
              PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE)
                .unwrap();
            let sdata_2 =
              to_bytes::<SPDPDiscoveredParticipantData, LittleEndian>(&participant_data_2)
                //to_little_endian_binary::<SPDPDiscoveredParticipantData>(&participant_data_2)
                .unwrap();
            // now the order of bytes should be the same
            assert_eq!(sdata, sdata_2);
          }

          _ => continue,
        },
        SubmessageBody::Interpreter(_) => (),
      }
    }
  }
}
