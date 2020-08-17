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

use std::{net::SocketAddr, time::Duration as StdDuration};
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
  // pub fn new() -> SPDPDiscoveredParticipantData {}

  pub fn as_reader_proxy(&self) -> Option<RtpsReaderProxy> {
    let remote_reader_guid = GUID::new_with_prefix_and_id(
      self.participant_guid.unwrap().guidPrefix,
      EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
    );
    let mut proxy = RtpsReaderProxy::new(remote_reader_guid);
    proxy.expects_in_line_qos = match self.expects_inline_qos {
      Some(v) => v,
      None => return None,
    };
    proxy.multicast_locator_list = self.default_multicast_locators.clone();
    proxy.unicast_locator_list = self.default_unicast_locators.clone();
    Some(proxy)
  }

  pub fn from_participant(participant: &DomainParticipant) -> SPDPDiscoveredParticipantData {
    let mut metatraffic_unicast_locators = LocatorList::new();
    let saddr = SocketAddr::new(
      "239.255.0.1".parse().unwrap(),
      get_spdp_well_known_multicast_port(participant.domain_id()),
    );
    metatraffic_unicast_locators.push(Locator::from(saddr));

    let mut metatraffic_multicast_locators = LocatorList::new();
    let saddr = SocketAddr::new(
      "0.0.0.0".parse().unwrap(),
      get_spdp_well_known_unicast_port(participant.domain_id(), participant.participant_id()),
    );
    metatraffic_multicast_locators.push(Locator::from(saddr));

    let mut default_unicast_locators = LocatorList::new();
    let saddr = SocketAddr::new(
      "239.255.0.1".parse().unwrap(),
      get_user_traffic_multicast_port(participant.domain_id()),
    );
    default_unicast_locators.push(Locator::from(saddr));

    let mut default_multicast_locators = LocatorList::new();
    let saddr = SocketAddr::new(
      "0.0.0.0".parse().unwrap(),
      get_user_traffic_unicast_port(participant.domain_id(), participant.participant_id()),
    );
    default_multicast_locators.push(Locator::from(saddr));

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
      available_builtin_endpoints: None,
      lease_duration: Some(Duration::from(StdDuration::from_secs(60))),
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
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_byte_buf(custom_ds).unwrap();
    Ok(res.generate_spdp_participant_data())
  }
}

impl Serialize for SPDPDiscoveredParticipantData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::new(&self);
    builtin_data_serializer.serialize::<S>(serializer)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::submessages::EntitySubmessage;
  use speedy::{Endianness, Readable};
  use crate::serialization::message::Message;
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
  use crate::serialization::cdrSerializer::to_little_endian_binary;
  use crate::test::test_data::*;

  #[test]
  fn pdata_deserialize_serialize() {
    let data = spdp_participant_data_raw();

    let rtpsmsg = Message::read_from_buffer_with_ctx(Endianness::LittleEndian, &data).unwrap();
    let submsgs = rtpsmsg.submessages();

    for submsg in submsgs.iter() {
      match submsg.submessage.as_ref() {
        Some(v) => match v {
          EntitySubmessage::Data(d, _) => {
            let participant_data: SPDPDiscoveredParticipantData =
              deserialize_from_little_endian(d.serialized_payload.value.clone()).unwrap();

            let sdata =
              to_little_endian_binary::<SPDPDiscoveredParticipantData>(&participant_data).unwrap();

            // order cannot be known at this point
            assert_eq!(sdata.len(), d.serialized_payload.value.len());

            let participant_data_2: SPDPDiscoveredParticipantData =
              deserialize_from_little_endian(sdata.clone()).unwrap();
            let sdata_2 =
              to_little_endian_binary::<SPDPDiscoveredParticipantData>(&participant_data_2)
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
