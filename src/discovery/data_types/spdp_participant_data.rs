use serde::{Serialize, Deserialize, de::Error};

use crate::{
  dds::{
    traits::{key::Keyed, Key},
    rtps_reader_proxy::RtpsReaderProxy,
    participant::DomainParticipant,
    rtps_writer_proxy::RtpsWriterProxy,
  },
  network::util::get_local_multicast_locators,
  network::util::get_local_unicast_socket_address,
  dds::qos::QosPolicies,
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
    builtin_data_serializer::BuiltinDataSerializer_Key,
    builtin_data_deserializer::BuiltinDataDeserializer,
  },
  network::constant::*,
};

//use std::{time::Duration as StdDuration};

use chrono::Utc;

// separate type is needed to serialize correctly
#[derive(Eq,PartialEq,Ord,PartialOrd,Debug, Clone, Copy, Hash)]
pub struct SPDPDiscoveredParticipantData_Key(pub GUID);

impl Key for SPDPDiscoveredParticipantData_Key {}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SPDPDiscoveredParticipantData {
  pub updated_time: chrono::DateTime<Utc>, 
  pub protocol_version: ProtocolVersion,
  pub vendor_id: VendorId,
  pub expects_inline_qos: bool,
  pub participant_guid: GUID,
  pub metatraffic_unicast_locators: LocatorList,
  pub metatraffic_multicast_locators: LocatorList,
  pub default_unicast_locators: LocatorList,
  pub default_multicast_locators: LocatorList,
  pub available_builtin_endpoints: BuiltinEndpointSet,
  pub lease_duration: Option<Duration>,
  pub manual_liveliness_count: i32,
  pub builtin_endpoint_qos: Option<BuiltinEndpointQos>,
  pub entity_name: Option<String>,
}

impl SPDPDiscoveredParticipantData {
  pub(crate) fn as_reader_proxy(
    &self,
    is_metatraffic: bool,
    entity_id: Option<EntityId>,
  ) -> RtpsReaderProxy {
    let remote_reader_guid = GUID::new_with_prefix_and_id(
      self.participant_guid.guidPrefix,
      match entity_id {
        Some(id) => id,
        None => EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
      },
    );

    let mut proxy = RtpsReaderProxy::new( remote_reader_guid,
        QosPolicies::qos_none() // TODO: What is the correct QoS value here?
      );
    proxy.expects_in_line_qos = self.expects_inline_qos;

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
      self.participant_guid.guidPrefix,
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

  pub fn from_local_participant(
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
      updated_time: Utc::now(),
      protocol_version: ProtocolVersion::PROTOCOLVERSION_2_3,
      vendor_id: VendorId::THIS_IMPLEMENTATION,
      expects_inline_qos: false,
      participant_guid: participant.get_guid(),
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


impl<'de> Deserialize<'de> for SPDPDiscoveredParticipantData {
  fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let visitor = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(visitor)?;
    res.generate_spdp_participant_data().map_err(|e| 
      D::Error::custom(format!("SPDPDiscoveredParticipantData::deserialize - {:?} - data was {:?}",e, &res) ))
  }
}

impl Serialize for SPDPDiscoveredParticipantData {
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_participant_data(self);
    builtin_data_serializer.serialize::<S>(serializer, true)
  }
}



impl Keyed for SPDPDiscoveredParticipantData {
  type K = SPDPDiscoveredParticipantData_Key; 
  fn get_key(&self) -> Self::K {
    SPDPDiscoveredParticipantData_Key( self.participant_guid ) 
  }
}

impl Serialize for SPDPDiscoveredParticipantData_Key {
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer_Key::from_data(*self);
    builtin_data_serializer.serialize::<S>(serializer, true)
  } 
}

impl<'de> Deserialize<'de> for SPDPDiscoveredParticipantData_Key {
  fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let visitor = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(visitor)?;
    res.generate_spdp_participant_data_key().map_err(|e| 
      D::Error::custom(format!("SPDPDiscoveredParticipantData_Key::deserialize - {:?} - data was {:?}",e, &res) ))
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
  use crate::dds::traits::serde_adapters::no_key::DeserializerAdapter;

  #[test]
  fn pdata_deserialize_serialize() {
    let data = spdp_participant_data_raw();

    let rtpsmsg = Message::read_from_buffer(data.clone()).unwrap();
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
            eprintln!("message data = {:?}",&data);
            eprintln!("payload    = {:?}", &d.serialized_payload.as_ref().unwrap().value.to_vec());
            eprintln!("deserialized  = {:?}", &participant_data);
            eprintln!("serialized = {:?}", &sdata);
            // order cannot be known at this point
            //assert_eq!(
            //  sdata.len(),
            //  d.serialized_payload.as_ref().unwrap().value.len()
            //);

            let mut participant_data_2: SPDPDiscoveredParticipantData =
              PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE)
                .unwrap();
            // force timestamps to be the same, as these are not serialized/deserialized, but
            // stamped during deserialization
            participant_data_2.updated_time = participant_data.updated_time;

            eprintln!("again deserialized = {:?}", &participant_data_2);
            let _sdata_2 =
              to_bytes::<SPDPDiscoveredParticipantData, LittleEndian>(&participant_data_2)
                .unwrap();
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
