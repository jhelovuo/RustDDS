use crate::{
  structure::{
    locator::LocatorData,
    parameter_id::ParameterId,
    guid::GUIDData,
    builtin_endpoint::{BuiltinEndpointQosData, BuiltinEndpointSetData},
    duration::DurationData,
  },
  discovery::data_types::spdp_participant_data::SPDPDiscoveredParticipantData,
  messages::{vendor_id::VendorIdData, protocol_version::ProtocolVersionData},
};
use serde::{Serialize, Serializer, ser::SerializeStruct, Deserialize};

#[derive(Serialize, Deserialize)]
struct ExpectsInlineQos {
  parameter_id: ParameterId,
  parameter_length: u16,
  expects_inline_qos: bool,
}

#[derive(Serialize, Deserialize)]
struct ManualLivelinessCount {
  parameter_id: ParameterId,
  parameter_length: u16,
  manual_liveliness_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct EntityName {
  parameter_id: ParameterId,
  parameter_length: u16,
  entity_name: String,
}

pub struct BuiltinDataSerializer<'a> {
  participant_data: &'a SPDPDiscoveredParticipantData,
}

impl<'a> BuiltinDataSerializer<'a> {
  pub fn new(participant_data: &'a SPDPDiscoveredParticipantData) -> BuiltinDataSerializer<'a> {
    BuiltinDataSerializer {
      participant_data: participant_data,
    }
  }

  pub fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut s = serializer
      .serialize_struct("SPDPParticipantData", self.fields_amount())
      .unwrap();

    self.add_protocol_version::<S>(&mut s);
    self.add_vendor_id::<S>(&mut s);
    self.add_expects_inline_qos::<S>(&mut s);
    self.add_participant_guid::<S>(&mut s);
    self.add_metatraffic_unicast_locators::<S>(&mut s);
    self.add_metatraffic_multicast_locators::<S>(&mut s);
    self.add_default_unicast_locators::<S>(&mut s);
    self.add_default_multicast_locators::<S>(&mut s);
    self.add_available_builtin_endpoint_set::<S>(&mut s);
    self.add_lease_duration::<S>(&mut s);
    self.add_manual_liveliness_count::<S>(&mut s);
    self.add_builtin_endpoint_qos::<S>(&mut s);
    self.add_entity_name::<S>(&mut s);

    s.serialize_field("sentinel", &(1 as u32)).unwrap();

    s.end()
  }

  fn fields_amount(&self) -> usize {
    let mut count: usize = 0;

    count = count + self.participant_data.protocol_version.is_some() as usize;
    count = count + self.participant_data.vendor_id.is_some() as usize;
    count = count + self.participant_data.expects_inline_qos.is_some() as usize;
    count = count + self.participant_data.participant_guid.is_some() as usize;
    count = count + self.participant_data.metatraffic_unicast_locators.len();
    count = count + self.participant_data.metatraffic_multicast_locators.len();
    count = count + self.participant_data.default_unicast_locators.len();
    count = count + self.participant_data.default_multicast_locators.len();
    count = count + self.participant_data.available_builtin_endpoints.is_some() as usize;
    count = count + self.participant_data.lease_duration.is_some() as usize;
    count = count + self.participant_data.manual_liveliness_count.is_some() as usize;
    count = count + self.participant_data.builtin_enpoint_qos.is_some() as usize;
    count = count + self.participant_data.entity_name.is_some() as usize;
    count
  }

  fn add_protocol_version<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    match &self.participant_data.protocol_version.as_ref() {
      Some(pv) => {
        s.serialize_field("protocol_version", &ProtocolVersionData::from(&pv))
          .unwrap();
      }
      None => (),
    }
  }

  fn add_vendor_id<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    match &self.participant_data.vendor_id.as_ref() {
      Some(vid) => {
        s.serialize_field("vendor_id", &VendorIdData::from(&vid))
          .unwrap();
      }
      None => (),
    }
  }

  fn add_expects_inline_qos<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    match &self.participant_data.expects_inline_qos.as_ref() {
      Some(iqos) => {
        let iq = ExpectsInlineQos {
          parameter_id: ParameterId::PID_EXPECTS_INLINE_QOS,
          parameter_length: 1,
          expects_inline_qos: *iqos.clone(),
        };
        s.serialize_field("expects_inline_qos", &iq).unwrap();
      }
      None => (),
    }
  }

  fn add_participant_guid<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    match &self.participant_data.participant_guid.as_ref() {
      Some(guid) => {
        s.serialize_field("participant_guid", &GUIDData::from(guid))
          .unwrap();
      }
      None => (),
    }
  }

  fn add_metatraffic_unicast_locators<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    // TODO: make sure this works with multiple locators
    for locator in self.participant_data.metatraffic_unicast_locators.iter() {
      s.serialize_field(
        "metatraffic_unicast_locators",
        &LocatorData::from(locator, ParameterId::PID_METATRAFFIC_UNICAST_LOCATOR),
      )
      .unwrap();
    }
  }

  fn add_metatraffic_multicast_locators<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    // TODO: make sure this works with multiple locators
    for locator in self.participant_data.metatraffic_multicast_locators.iter() {
      s.serialize_field(
        "metatraffic_multicast_locators",
        &LocatorData::from(locator, ParameterId::PID_METATRAFFIC_MULTICAST_LOCATOR),
      )
      .unwrap();
    }
  }

  fn add_default_unicast_locators<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    // TODO: make sure this works with multiple locators
    for locator in self.participant_data.default_unicast_locators.iter() {
      s.serialize_field(
        "default_unicast_locators",
        &LocatorData::from(locator, ParameterId::PID_DEFAULT_UNICAST_LOCATOR),
      )
      .unwrap();
    }
  }

  fn add_default_multicast_locators<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    // TODO: make sure this works with multiple locators
    for locator in self.participant_data.default_multicast_locators.iter() {
      s.serialize_field(
        "default_multicast_locators",
        &LocatorData::from(locator, ParameterId::PID_DEFAULT_MULTICAST_LOCATOR),
      )
      .unwrap();
    }
  }

  fn add_available_builtin_endpoint_set<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    match &self.participant_data.available_builtin_endpoints.as_ref() {
      Some(eps) => {
        s.serialize_field(
          "available_builtin_endpoints",
          &BuiltinEndpointSetData::from(eps, &ParameterId::PID_BUILTIN_ENDPOINT_SET),
        )
        .unwrap();
      }
      None => (),
    }
  }

  fn add_lease_duration<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    match &self.participant_data.lease_duration.as_ref() {
      Some(dur) => {
        s.serialize_field("lease_duration", &DurationData::from(dur))
          .unwrap();
      }
      None => (),
    }
  }

  fn add_manual_liveliness_count<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    match &self.participant_data.manual_liveliness_count.as_ref() {
      Some(liv) => {
        let mliv = ManualLivelinessCount {
          parameter_id: ParameterId::PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT,
          parameter_length: 4,
          manual_liveliness_count: *liv.clone(),
        };
        s.serialize_field("manual_liveliness_count", &mliv).unwrap();
      }
      None => (),
    }
  }

  fn add_builtin_endpoint_qos<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    match &self.participant_data.builtin_enpoint_qos.as_ref() {
      Some(qos) => {
        s.serialize_field("builtin_endpoint_qos", &BuiltinEndpointQosData::from(qos))
          .unwrap();
      }
      None => (),
    }
  }

  fn add_entity_name<S: Serializer>(&self, s: &mut S::SerializeStruct) {
    match &self.participant_data.entity_name.as_ref() {
      Some(name) => {
        let ename = EntityName {
          parameter_id: ParameterId::PID_ENTITY_NAME,
          // adding 4 bytes for string lenght and 1 byte for terminate
          parameter_length: name.len() as u16 + 5,
          entity_name: name.to_string(),
        };
        s.serialize_field("entity_name", &ename).unwrap();
      }
      None => (),
    }
  }
}
