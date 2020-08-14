use speedy::Readable;

use crate::structure::{
  guid::GUID,
  parameter_id::ParameterId,
  locator::{LocatorList, Locator},
  builtin_endpoint::{BuiltinEndpointSet, BuiltinEndpointQos},
  duration::Duration,
};

use crate::messages::{protocol_version::ProtocolVersion, vendor_id::VendorId};

use crate::serialization::{cdrDeserializer::deserialize_from_little_endian, error::Error};

use crate::discovery::data_types::spdp_participant_data::SPDPDiscoveredParticipantData;

#[derive(Debug)]
pub struct BuiltinDataDeserializer {
  buffer: Vec<u8>,
  protocol_version: Option<ProtocolVersion>,
  vendor_id: Option<VendorId>,
  expects_inline_qos: Option<bool>,
  participant_guid: Option<GUID>,
  metatraffic_unicast_locators: LocatorList,
  metatraffic_multicast_locators: LocatorList,
  default_unicast_locators: LocatorList,
  default_multicast_locators: LocatorList,
  available_builtin_endpoints: Option<BuiltinEndpointSet>,
  lease_duration: Option<Duration>,
  manual_liveliness_count: Option<i32>,
  builtin_enpoint_qos: Option<BuiltinEndpointQos>,
  entity_name: Option<String>,
  sentinel: Option<u16>,
}

impl BuiltinDataDeserializer {
  pub fn new(buffer: Vec<u8>) -> BuiltinDataDeserializer {
    BuiltinDataDeserializer {
      buffer,
      protocol_version: None,
      vendor_id: None,
      expects_inline_qos: None,
      participant_guid: None,
      metatraffic_unicast_locators: LocatorList::new(),
      metatraffic_multicast_locators: LocatorList::new(),
      default_unicast_locators: LocatorList::new(),
      default_multicast_locators: LocatorList::new(),
      available_builtin_endpoints: None,
      lease_duration: None,
      manual_liveliness_count: None,
      builtin_enpoint_qos: None,
      entity_name: None,
      sentinel: None,
    }
  }

  pub fn generate_spdp_participant_data(self) -> SPDPDiscoveredParticipantData {
    SPDPDiscoveredParticipantData {
      updated_time: time::precise_time_ns(),
      protocol_version: self.protocol_version,
      vendor_id: self.vendor_id,
      expects_inline_qos: self.expects_inline_qos,
      participant_guid: self.participant_guid,
      metatraffic_unicast_locators: self.metatraffic_unicast_locators,
      metatraffic_multicast_locators: self.metatraffic_multicast_locators,
      default_unicast_locators: self.default_unicast_locators,
      default_multicast_locators: self.default_multicast_locators,
      available_builtin_endpoints: self.available_builtin_endpoints,
      lease_duration: self.lease_duration,
      manual_liveliness_count: self.manual_liveliness_count,
      builtin_enpoint_qos: self.builtin_enpoint_qos,
      entity_name: self.entity_name,
    }
  }

  pub fn parse_data(mut self) -> BuiltinDataDeserializer {
    while self.sentinel.is_none() && self.buffer.len() > 0 {
      self = self.read_next();
    }

    self
  }

  pub fn read_next(mut self) -> BuiltinDataDeserializer {
    let parameter_id = BuiltinDataDeserializer::read_parameter_id(&self.buffer).unwrap();
    let parameter_length: usize =
      BuiltinDataDeserializer::read_parameter_length(&self.buffer).unwrap() as usize;
    match parameter_id {
      ParameterId::PID_PARTICIPANT_GUID => {
        let guid: Result<GUID, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match guid {
          Ok(gg) => {
            self.participant_guid = Some(gg);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_PROTOCOL_VERSION => {
        let version: Result<ProtocolVersion, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match version {
          Ok(vv) => {
            self.protocol_version = Some(vv);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_VENDOR_ID => {
        let vendor: Result<VendorId, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match vendor {
          Ok(vv) => {
            self.vendor_id = Some(vv);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_EXPECTS_INLINE_QOS => {
        let inline_qos: Result<bool, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match inline_qos {
          Ok(qos) => {
            self.expects_inline_qos = Some(qos);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_METATRAFFIC_UNICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match locator {
          Ok(loc) => {
            self.metatraffic_unicast_locators.push(loc);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_METATRAFFIC_MULTICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match locator {
          Ok(loc) => {
            self.metatraffic_multicast_locators.push(loc);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_DEFAULT_UNICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match locator {
          Ok(loc) => {
            self.default_unicast_locators.push(loc);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_DEFAULT_MULTICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match locator {
          Ok(loc) => {
            self.default_multicast_locators.push(loc);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_BUILTIN_ENDPOINT_SET => {
        let endpoints: Result<BuiltinEndpointSet, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match endpoints {
          Ok(ep) => {
            self.available_builtin_endpoints = Some(ep);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_PARTICIPANT_LEASE_DURATION => {
        let duration: Result<Duration, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match duration {
          Ok(dur) => {
            self.lease_duration = Some(dur);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT => {
        let count: Result<i32, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match count {
          Ok(c) => {
            self.manual_liveliness_count = Some(c);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_BUILTIN_ENDPOINT_QOS => {
        let qos: Result<BuiltinEndpointQos, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match qos {
          Ok(q) => {
            self.builtin_enpoint_qos = Some(q);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_ENTITY_NAME => {
        let name: Result<String, Error> =
          deserialize_from_little_endian(self.buffer[4..4 + parameter_length].to_vec());
        match name {
          Ok(n) => {
            self.entity_name = Some(n);
            self.buffer = self.buffer[4 + parameter_length..].to_vec();
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_SENTINEL => {
        self.sentinel = Some(1);
        self.buffer = Vec::new();
        return self;
      }
      _ => (),
    }
    // let serializer = u
    self.buffer = self.buffer[4 + parameter_length..].to_vec();
    self
  }

  fn read_parameter_id(buffer: &Vec<u8>) -> Option<ParameterId> {
    let par: Result<ParameterId, Error> = deserialize_from_little_endian(buffer[..2].to_vec());
    match par {
      Ok(val) => Some(val),
      _ => None,
    }
  }

  fn read_parameter_length(buffer: &Vec<u8>) -> Option<u16> {
    if buffer.len() < 4 {
      return Some(0);
    }

    let parameter_length_value = u16::read_from_buffer(&buffer[2..4]);
    match parameter_length_value {
      Ok(val) => Some(val),
      _ => None,
    }
  }
}
