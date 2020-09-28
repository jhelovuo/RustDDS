use serde::Deserialize;

use crate::structure::{
  guid::GUID,
  parameter_id::ParameterId,
  locator::{LocatorList, Locator},
  builtin_endpoint::{BuiltinEndpointSet, BuiltinEndpointQos},
  duration::Duration,
  endpoint::ReliabilityKind,
};

use crate::messages::{
  protocol_version::ProtocolVersion, vendor_id::VendorId,
  submessages::submessage_elements::serialized_payload::RepresentationIdentifier,
};

use crate::serialization::error::Error;

use crate::{
  dds::{
    qos::policy::{
      Deadline, Durability, LatencyBudget, Liveliness, Reliability, Ownership, DestinationOrder,
      TimeBasedFilter, Presentation, Lifespan, History, ResourceLimits,
    },
    traits::serde_adapters::DeserializerAdapter,
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

use super::cdrDeserializer::CDR_deserializer_adapter;

#[derive(Debug)]
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

  pub fn generate_spdp_participant_data(&self) -> SPDPDiscoveredParticipantData {
    SPDPDiscoveredParticipantData {
      updated_time: time::precise_time_ns(),
      protocol_version: self.protocol_version,
      vendor_id: self.vendor_id,
      expects_inline_qos: self.expects_inline_qos,
      participant_guid: self.participant_guid,
      metatraffic_unicast_locators: self.metatraffic_unicast_locators.clone(),
      metatraffic_multicast_locators: self.metatraffic_multicast_locators.clone(),
      default_unicast_locators: self.default_unicast_locators.clone(),
      default_multicast_locators: self.default_multicast_locators.clone(),
      available_builtin_endpoints: self.available_builtin_endpoints.clone(),
      lease_duration: self.lease_duration,
      manual_liveliness_count: self.manual_liveliness_count,
      builtin_enpoint_qos: self.builtin_enpoint_qos,
      entity_name: self.entity_name.clone(),
    }
  }

  pub fn generate_reader_proxy(&self) -> ReaderProxy {
    ReaderProxy {
      remote_reader_guid: self.endpoint_guid,
      expects_inline_qos: self.expects_inline_qos,
      unicast_locator_list: self.unicast_locator_list.clone(),
      multicast_locator_list: self.multicast_locator_list.clone(),
    }
  }

  pub fn generate_writer_proxy(&self) -> WriterProxy {
    WriterProxy {
      remote_writer_guid: self.endpoint_guid,
      unicast_locator_list: self.unicast_locator_list.clone(),
      multicast_locator_list: self.multicast_locator_list.clone(),
      data_max_size_serialized: self.data_max_size_serialized,
    }
  }

  pub fn generate_subscription_topic_data(&self) -> SubscriptionBuiltinTopicData {
    SubscriptionBuiltinTopicData {
      key: self.endpoint_guid,
      participant_key: self.participant_guid,
      topic_name: self.topic_name.clone(),
      type_name: self.type_name.clone(),
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

  pub fn generate_publication_topic_data(&self) -> PublicationBuiltinTopicData {
    PublicationBuiltinTopicData {
      key: self.endpoint_guid,
      participant_key: self.participant_guid,
      topic_name: self.topic_name.clone(),
      type_name: self.type_name.clone(),
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
    let reader_proxy = self.generate_reader_proxy();
    let subscription_topic_data = self.generate_subscription_topic_data();
    DiscoveredReaderData {
      reader_proxy,
      subscription_topic_data,
      content_filter: self.content_filter_property,
    }
  }

  pub fn generate_discovered_writer_data(self) -> DiscoveredWriterData {
    let writer_proxy = self.generate_writer_proxy();
    let publication_topic_data = self.generate_publication_topic_data();
    DiscoveredWriterData {
      writer_proxy,
      publication_topic_data,
    }
  }

  pub fn parse_data_little_endian(self, buffer: &[u8]) -> BuiltinDataDeserializer {
    self.parse_data(buffer, RepresentationIdentifier::CDR_LE)
  }

  pub fn parse_data_big_endian(self, buffer: &[u8]) -> BuiltinDataDeserializer {
    self.parse_data(buffer, RepresentationIdentifier::CDR_BE)
  }

  fn parse_data(mut self, buffer: &[u8], rep: RepresentationIdentifier) -> BuiltinDataDeserializer {
    let mut buffer = buffer.to_vec();
    while self.sentinel.is_none() && buffer.len() > 0 {
      self = self.read_next(&mut buffer, rep);
    }

    self
  }

  pub fn read_next(
    mut self,
    buffer: &mut Vec<u8>,
    rep: RepresentationIdentifier,
  ) -> BuiltinDataDeserializer {
    let parameter_id = BuiltinDataDeserializer::read_parameter_id(&buffer, rep).unwrap();
    let mut parameter_length: usize =
      BuiltinDataDeserializer::read_parameter_length(&buffer, rep).unwrap() as usize;

    if (parameter_length + 4) > buffer.len() {
      parameter_length = buffer.len() - 4;
    }

    match parameter_id {
      ParameterId::PID_PARTICIPANT_GUID => {
        let guid: Result<GUID, Error> =
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
        #[derive(Deserialize, Clone)]
        struct ReliabilityBestEffortData {
          pub reliability_kind: ReliabilityKind,
          pub max_blocking_time: Duration,
        }

        let reliability: Result<ReliabilityBestEffortData, Error> =
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        match reliability {
          Ok(rel) => {
            self.reliability = match rel.reliability_kind {
              ReliabilityKind::BEST_EFFORT => Some(Reliability::BestEffort),
              ReliabilityKind::RELIABLE => Some(Reliability::Reliable {
                max_blocking_time: rel.max_blocking_time,
              }),
              _ => {
                buffer.drain(..4 + parameter_length);
                return self;
              }
            };

            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_OWNERSHIP => {
        #[derive(Deserialize)]
        enum OwnershipKind {
          SHARED,
          EXCLUSIVE,
        }
        let ownership: Result<OwnershipKind, Error> =
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        match ownership {
          Ok(own) => {
            let strength = match self.ownership {
              Some(v) => match v {
                Ownership::Exclusive { strength } => strength,
                _ => 0,
              },
              None => 0,
            };

            let own = match own {
              OwnershipKind::SHARED => Ownership::Shared,
              OwnershipKind::EXCLUSIVE => Ownership::Exclusive { strength: strength },
            };

            self.ownership = Some(own);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_OWNERSHIP_STRENGTH => {
        let ownership_strength: Result<i32, Error> =
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        match ownership_strength {
          Ok(stri) => {
            self.ownership = match self.ownership {
              Some(v) => match v {
                Ownership::Exclusive { strength: _ } => {
                  Some(Ownership::Exclusive { strength: stri })
                }
                _ => Some(v),
              },
              None => Some(Ownership::Exclusive { strength: stri }),
            }
          }
          _ => (),
        };
        buffer.drain(..4 + parameter_length);
        return self;
      }
      ParameterId::PID_DESTINATION_ORDER => {
        let destination_order: Result<DestinationOrder, Error> =
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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
        #[derive(Deserialize)]
        enum HistoryKind {
          KEEP_LAST,
          KEEP_ALL,
        }

        #[derive(Deserialize)]
        struct HistoryData {
          pub kind: HistoryKind,
          pub depth: i32,
        }

        let history: Result<HistoryData, Error> =
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        match history {
          Ok(his) => {
            let h = match his.kind {
              HistoryKind::KEEP_LAST => History::KeepLast { depth: his.depth },
              HistoryKind::KEEP_ALL => History::KeepAll,
            };
            self.history = Some(h);
            buffer.drain(..4 + parameter_length);
            return self;
          }
          _ => (),
        }
      }
      ParameterId::PID_RESOURCE_LIMITS => {
        let resource_limits: Result<ResourceLimits, Error> =
          CDR_deserializer_adapter::from_bytes(&buffer[4..4 + parameter_length], rep);
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

  pub fn read_parameter_id(buffer: &Vec<u8>, rep: RepresentationIdentifier) -> Option<ParameterId> {
    let par: Result<ParameterId, Error> = CDR_deserializer_adapter::from_bytes(&buffer[..2], rep);
    match par {
      Ok(val) => Some(val),
      _ => None,
    }
  }

  pub fn read_parameter_length(buffer: &Vec<u8>, rep: RepresentationIdentifier) -> Option<u16> {
    if buffer.len() < 4 {
      return Some(0);
    }

    let parameter_length_value = CDR_deserializer_adapter::from_bytes(&buffer[2..4], rep);
    match parameter_length_value {
      Ok(val) => Some(val),
      _ => None,
    }
  }
}
