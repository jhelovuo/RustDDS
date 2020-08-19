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

use crate::{
  dds::{
    qos::policy::{
      Deadline, Durability, LatencyBudget, Liveliness, Reliability, Ownership, DestinationOrder,
      TimeBasedFilter, Presentation, Lifespan, History, ResourceLimits,
    },
  },
  discovery::{
    content_filter_property::ContentFilterProperty,
    data_types::{
      topic_data::{
        SubscriptionBuiltinTopicData, ReaderProxy, DiscoveredReaderData, WriterProxy,
        PublicationBuiltinTopicData, DiscoveredWriterData, TopicBuiltinTopicData,
      },
      spdp_participant_data::SPDPDiscoveredParticipantData,
    },
  },
};

#[derive(Debug, Clone)]
pub struct BuiltinDataDeserializer {
  // Participant Data
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
  pub sentinel: Option<u32>,

  pub endpoint_guid: Option<GUID>,

  // Reader Proxy
  pub unicast_locator_list: LocatorList,
  pub multicast_locator_list: LocatorList,

  // Writer Proxy
  pub data_max_size_serialized: Option<u32>,

  // topic data
  pub topic_name: Option<String>,
  pub type_name: Option<String>,
  pub durability: Option<Durability>,
  pub deadline: Option<Deadline>,
  pub latency_budget: Option<LatencyBudget>,
  pub liveliness: Option<Liveliness>,
  pub reliability: Option<Reliability>,
  pub ownership: Option<Ownership>,
  pub destination_order: Option<DestinationOrder>,
  pub time_based_filter: Option<TimeBasedFilter>,
  pub presentation: Option<Presentation>,
  pub lifespan: Option<Lifespan>,
  pub history: Option<History>,
  pub resource_limits: Option<ResourceLimits>,

  pub content_filter_property: Option<ContentFilterProperty>,
}

impl BuiltinDataDeserializer {
  pub fn new() -> BuiltinDataDeserializer {
    BuiltinDataDeserializer {
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

      endpoint_guid: None,
      unicast_locator_list: LocatorList::new(),
      multicast_locator_list: LocatorList::new(),

      data_max_size_serialized: None,

      topic_name: None,
      type_name: None,
      durability: None,
      deadline: None,
      latency_budget: None,
      liveliness: None,
      reliability: None,
      ownership: None,
      destination_order: None,
      time_based_filter: None,
      presentation: None,
      lifespan: None,
      history: None,
      resource_limits: None,

      content_filter_property: None,
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

  pub fn generate_reader_proxy(self) -> ReaderProxy {
    ReaderProxy {
      remote_reader_guid: self.endpoint_guid,
      expects_inline_qos: self.expects_inline_qos,
      unicast_locator_list: self.unicast_locator_list,
      multicast_locator_list: self.multicast_locator_list,
    }
  }

  pub fn generate_writer_proxy(self) -> WriterProxy {
    WriterProxy {
      remote_writer_guid: self.endpoint_guid,
      unicast_locator_list: self.unicast_locator_list,
      multicast_locator_list: self.multicast_locator_list,
      data_max_size_serialized: self.data_max_size_serialized,
    }
  }

  pub fn generate_subscription_topic_data(self) -> SubscriptionBuiltinTopicData {
    SubscriptionBuiltinTopicData {
      key: self.endpoint_guid,
      participant_key: self.participant_guid,
      topic_name: self.topic_name,
      type_name: self.type_name,
      durability: self.durability,
      deadline: self.deadline,
      latency_budget: self.latency_budget,
      liveliness: self.liveliness,
      reliability: self.reliability,
      ownership: self.ownership,
      destination_order: self.destination_order,
      time_based_filter: self.time_based_filter,
      presentation: self.presentation,
      lifespan: self.lifespan,
    }
  }

  pub fn generate_publication_topic_data(self) -> PublicationBuiltinTopicData {
    PublicationBuiltinTopicData {
      key: self.endpoint_guid,
      participant_key: self.participant_guid,
      topic_name: self.topic_name,
      type_name: self.type_name,
      durability: self.durability,
      deadline: self.deadline,
      latency_budget: self.latency_budget,
      liveliness: self.liveliness,
      reliability: self.reliability,
      lifespan: self.lifespan,
      time_based_filter: self.time_based_filter,
      ownership: self.ownership,
      destination_order: self.destination_order,
      presentation: self.presentation,
    }
  }

  pub fn generate_topic_data(self) -> TopicBuiltinTopicData {
    TopicBuiltinTopicData {
      key: self.endpoint_guid,
      name: self.topic_name,
      type_name: self.type_name,
      durability: self.durability,
      deadline: self.deadline,
      latency_budget: self.latency_budget,
      liveliness: self.liveliness,
      reliability: self.reliability,
      lifespan: self.lifespan,
      destination_order: self.destination_order,
      presentation: self.presentation,
      history: self.history,
      resource_limits: self.resource_limits,
      ownership: self.ownership,
    }
  }

  pub fn generate_discovered_reader_data(self) -> DiscoveredReaderData {
    // TODO: refactor so that clones are not necessary
    let reader_proxy = self.clone().generate_reader_proxy();
    let subscription_topic_data = self.clone().generate_subscription_topic_data();
    DiscoveredReaderData {
      reader_proxy,
      subscription_topic_data,
      content_filter: self.content_filter_property,
    }
  }

  pub fn generate_discovered_writer_data(self) -> DiscoveredWriterData {
    // TODO: refactor so that clones are not necessary
    let writer_proxy = self.clone().generate_writer_proxy();
    let publication_topic_data = self.clone().generate_publication_topic_data();
    DiscoveredWriterData {
      writer_proxy,
      publication_topic_data,
    }
  }

  pub fn parse_data(mut self, mut buffer: Vec<u8>) -> BuiltinDataDeserializer {
    while self.sentinel.is_none() && buffer.len() > 0 {
      self = self.read_next(&mut buffer);
    }

    self
  }

  pub fn read_next(mut self, buffer: &mut Vec<u8>) -> BuiltinDataDeserializer {
    let parameter_id = BuiltinDataDeserializer::read_parameter_id(&buffer).unwrap();
    let mut parameter_length: usize =
      BuiltinDataDeserializer::read_parameter_length(&buffer).unwrap() as usize;

    if (parameter_length + 4) > buffer.len() {
      parameter_length = buffer.len() - 4;
    }
    // TODO: decrease/remove copying of the buffer

    match parameter_id {
      ParameterId::PID_PARTICIPANT_GUID => {
        let guid: Result<GUID, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match guid {
          Ok(gg) => {
            self.participant_guid = Some(gg);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_PROTOCOL_VERSION => {
        let version: Result<ProtocolVersion, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match version {
          Ok(vv) => {
            self.protocol_version = Some(vv);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_VENDOR_ID => {
        let vendor: Result<VendorId, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match vendor {
          Ok(vv) => {
            self.vendor_id = Some(vv);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_EXPECTS_INLINE_QOS => {
        let inline_qos: Result<bool, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match inline_qos {
          Ok(qos) => {
            self.expects_inline_qos = Some(qos);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_METATRAFFIC_UNICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match locator {
          Ok(loc) => {
            self.metatraffic_unicast_locators.push(loc);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_METATRAFFIC_MULTICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match locator {
          Ok(loc) => {
            self.metatraffic_multicast_locators.push(loc);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_DEFAULT_UNICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match locator {
          Ok(loc) => {
            self.default_unicast_locators.push(loc);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_DEFAULT_MULTICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match locator {
          Ok(loc) => {
            self.default_multicast_locators.push(loc);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_BUILTIN_ENDPOINT_SET => {
        let endpoints: Result<BuiltinEndpointSet, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match endpoints {
          Ok(ep) => {
            self.available_builtin_endpoints = Some(ep);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_PARTICIPANT_LEASE_DURATION => {
        let duration: Result<Duration, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match duration {
          Ok(dur) => {
            self.lease_duration = Some(dur);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT => {
        let count: Result<i32, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match count {
          Ok(c) => {
            self.manual_liveliness_count = Some(c);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_BUILTIN_ENDPOINT_QOS => {
        let qos: Result<BuiltinEndpointQos, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match qos {
          Ok(q) => {
            self.builtin_enpoint_qos = Some(q);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_ENTITY_NAME => {
        let name: Result<String, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match name {
          Ok(n) => {
            self.entity_name = Some(n);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_ENDPOINT_GUID => {
        let guid: Result<GUID, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match guid {
          Ok(gg) => {
            self.endpoint_guid = Some(gg);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_UNICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match locator {
          Ok(loc) => {
            self.unicast_locator_list.push(loc);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_MULTICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match locator {
          Ok(loc) => {
            self.multicast_locator_list.push(loc);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_TOPIC_NAME => {
        let topic_name: Result<String, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match topic_name {
          Ok(name) => {
            self.topic_name = Some(name);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_TYPE_NAME => {
        let type_name: Result<String, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match type_name {
          Ok(name) => {
            self.type_name = Some(name);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_DURABILITY => {
        let durability: Result<Durability, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match durability {
          Ok(dur) => {
            self.durability = Some(dur);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_DEADLINE => {
        let deadline: Result<Deadline, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match deadline {
          Ok(dl) => {
            self.deadline = Some(dl);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_LATENCY_BUDGET => {
        let latency_budget: Result<LatencyBudget, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match latency_budget {
          Ok(lb) => {
            self.latency_budget = Some(lb);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_LIVELINESS => {
        let liveliness: Result<Liveliness, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match liveliness {
          Ok(liv) => {
            self.liveliness = Some(liv);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_RELIABILITY => {
        let reliability: Result<Reliability, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match reliability {
          Ok(rel) => {
            self.reliability = Some(rel);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_OWNERSHIP => {
        let ownership: Result<Ownership, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match ownership {
          Ok(own) => {
            self.ownership = Some(own);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_DESTINATION_ORDER => {
        let destination_order: Result<DestinationOrder, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match destination_order {
          Ok(deor) => {
            self.destination_order = Some(deor);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_TIME_BASED_FILTER => {
        let time_based_filter: Result<TimeBasedFilter, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match time_based_filter {
          Ok(tbf) => {
            self.time_based_filter = Some(tbf);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_PRESENTATION => {
        let presentation: Result<Presentation, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match presentation {
          Ok(p) => {
            self.presentation = Some(p);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_LIFESPAN => {
        let lifespan: Result<Lifespan, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match lifespan {
          Ok(ls) => {
            self.lifespan = Some(ls);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_CONTENT_FILTER_PROPERTY => {
        let content_filter: Result<ContentFilterProperty, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match content_filter {
          Ok(cfp) => {
            self.content_filter_property = Some(cfp);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_TYPE_MAX_SIZE_SERIALIZED => {
        let max_size: Result<u32, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match max_size {
          Ok(ms) => {
            self.data_max_size_serialized = Some(ms);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_HISTORY => {
        let history: Result<History, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match history {
          Ok(his) => {
            self.history = Some(his);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_RESOURCE_LIMITS => {
        let resource_limits: Result<ResourceLimits, Error> =
          deserialize_from_little_endian(&buffer[4..4 + parameter_length].to_vec());
        match resource_limits {
          Ok(lim) => {
            self.resource_limits = Some(lim);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_SENTINEL => {
        self.sentinel = Some(1);
        buffer.clear();
        return self;
      }
      _ => (),
    }

    buffer.drain(..4 + parameter_length);
    self
  }

  pub fn read_parameter_id(buffer: &Vec<u8>) -> Option<ParameterId> {
    let par: Result<ParameterId, Error> = deserialize_from_little_endian(&buffer[..2].to_vec());
    match par {
      Ok(val) => Some(val),
      _ => None,
    }
  }

  pub fn read_parameter_length(buffer: &Vec<u8>) -> Option<u16> {
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
