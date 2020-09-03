use serde::{Serialize, Deserialize};

use crate::dds::{
  traits::{
    key::{Key, Keyed},
  },
  rtps_reader_proxy::RtpsReaderProxy,
  participant::DomainParticipant,
};

use crate::messages::{protocol_version::ProtocolVersion, vendor_id::VendorId};
use crate::{
  structure::{
    locator::{Locator, LocatorList},
    guid::{EntityId, GUID},
    duration::Duration,
    builtin_endpoint::{BuiltinEndpointSet, BuiltinEndpointQos},
    entity::Entity,
  },
};

use crate::{
  serialization::{
    builtin_data_serializer::BuiltinDataSerializer,
    builtin_data_deserializer::BuiltinDataDeserializer,
  },
  network::constant::*,
};

use std::{
  net::{IpAddr, SocketAddr},
  time::Duration as StdDuration,
  io::Error,
};
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
  pub fn as_reader_proxy(&self, is_metatraffic: bool) -> RtpsReaderProxy {
    let remote_reader_guid = GUID::new_with_prefix_and_id(
      self.participant_guid.unwrap().guidPrefix,
      EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
    );
    let mut proxy = RtpsReaderProxy::new(remote_reader_guid);
    proxy.expects_in_line_qos = match self.expects_inline_qos {
      Some(v) => v,
      None => false,
    };

    if !is_metatraffic {
      proxy.multicast_locator_list = self.default_multicast_locators.clone();
      proxy.unicast_locator_list = self.default_unicast_locators.clone();
    } else {
      proxy.multicast_locator_list = self.metatraffic_multicast_locators.clone();
      proxy.unicast_locator_list = self.metatraffic_unicast_locators.clone();
    }

    proxy
  }

  pub fn from_participant(
    participant: &DomainParticipant,
    lease_duration: StdDuration,
  ) -> SPDPDiscoveredParticipantData {
    let mut metatraffic_multicast_locators = LocatorList::new();
    let saddr = SocketAddr::new(
      "239.255.0.1".parse().unwrap(),
      get_spdp_well_known_multicast_port(participant.domain_id()),
    );
    metatraffic_multicast_locators.push(Locator::from(saddr));

    let local_ips: Result<Vec<IpAddr>, Error> = get_if_addrs::get_if_addrs().map(|p| {
      p.iter()
        .filter(|ip| !ip.is_loopback())
        .map(|ip| ip.ip())
        .collect()
    });

    let metatraffic_unicast_locators: LocatorList = match local_ips {
      Ok(ips) => ips
        .iter()
        .map(|p| {
          Locator::from(SocketAddr::new(
            p.clone(),
            get_spdp_well_known_unicast_port(participant.domain_id(), participant.participant_id()),
          ))
        })
        .collect(),
      _ => Vec::new(),
    };

    let mut default_multicast_locators = LocatorList::new();
    let saddr = SocketAddr::new(
      "239.255.0.1".parse().unwrap(),
      get_user_traffic_multicast_port(participant.domain_id()),
    );
    default_multicast_locators.push(Locator::from(saddr));

    let local_ips: Result<Vec<IpAddr>, Error> = get_if_addrs::get_if_addrs().map(|p| {
      p.iter()
        .filter(|ip| !ip.is_loopback())
        .map(|ip| ip.ip())
        .collect()
    });

    let default_unicast_locators: LocatorList = match local_ips {
      Ok(ips) => ips
        .iter()
        .map(|p| {
          Locator::from(SocketAddr::new(
            p.clone(),
            get_user_traffic_unicast_port(participant.domain_id(), participant.participant_id()),
          ))
        })
        .collect(),
      _ => Vec::new(),
    };

    let builtin_endpoints = BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PARTICIPANT_ANNOUNCER
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_TOPICS_ANNOUNCER
      | BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_TOPICS_DETECTOR;

    SPDPDiscoveredParticipantData {
      updated_time: time::precise_time_ns(),
      protocol_version: Some(ProtocolVersion::PROTOCOLVERSION),
      vendor_id: Some(VendorId::VENDOR_UNKNOWN),
      expects_inline_qos: Some(false),
      participant_guid: Some(participant.get_guid().clone()),
      metatraffic_unicast_locators,
      metatraffic_multicast_locators,
      default_unicast_locators,
      default_multicast_locators,
      available_builtin_endpoints: Some(BuiltinEndpointSet::from(builtin_endpoints)),
      lease_duration: Some(Duration::from(lease_duration)),
      manual_liveliness_count: None,
      builtin_enpoint_qos: None,
      entity_name: None,
    }
  }
}

impl Keyed for SPDPDiscoveredParticipantData {
  type K = u64; // placeholder
  fn get_key(&self) -> Self::K {
    self.updated_time
  }
}

impl Key for u64 {}

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

  use crate::submessages::EntitySubmessage;
  use crate::serialization::message::Message;
  use crate::serialization::pl_cdr_deserializer::PlCdrDeserializerAdapter;
  use crate::serialization::cdrSerializer::{to_bytes};
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
      match submsg.submessage.as_ref() {
        Some(v) => match v {
          EntitySubmessage::Data(d, _) => {
            let participant_data: SPDPDiscoveredParticipantData =
              PlCdrDeserializerAdapter::from_bytes(
                &d.serialized_payload.value,
                RepresentationIdentifier::PL_CDR_LE,
              )
              .unwrap();

            let sdata =
              to_bytes::<SPDPDiscoveredParticipantData, LittleEndian>(&participant_data).unwrap();

            // order cannot be known at this point
            assert_eq!(sdata.len(), d.serialized_payload.value.len());

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
        None => (),
      }
    }

    // let serializer = cdrDeserializer::CDR_deserializer::deserialize_from_little_endian(data);
    // let res = data.serialize(serializer);
    // println!("AAAA: {:?}", res);
  }
}
