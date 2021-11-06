use std::time::Instant;

use serde::Deserialize;
use chrono::Utc;
use log::{error, warn};

use crate::{
  dds::{
    qos::{
      policy::{
        Deadline, DestinationOrder, Durability, History, LatencyBudget, Lifespan, Liveliness,
        Ownership, Presentation, Reliability, ResourceLimits, TimeBasedFilter,
      },
      QosPolicyBuilder,
    },
    traits::serde_adapters::no_key::*,
  },
  discovery::{
    content_filter_property::ContentFilterProperty,
    data_types::{
      spdp_participant_data::{SpdpDiscoveredParticipantData, SpdpDiscoveredParticipantDataKey},
      topic_data::{
        DiscoveredReaderData, DiscoveredReaderDataKey, DiscoveredWriterData,
        DiscoveredWriterDataKey, PublicationBuiltinTopicData, ReaderProxy,
        SubscriptionBuiltinTopicData, TopicBuiltinTopicData, WriterProxy,
      },
    },
  },
  log_and_err_discovery,
  messages::{
    protocol_version::ProtocolVersion,
    submessages::submessage_elements::serialized_payload::RepresentationIdentifier,
    vendor_id::VendorId,
  },
  serialization::error::Error,
  structure::{
    builtin_endpoint::{BuiltinEndpointQos, BuiltinEndpointSet},
    duration::Duration,
    endpoint::ReliabilityKind,
    guid::GUID,
    locator::{Locator, LocatorList},
    parameter_id::ParameterId,
  },
};
use super::cdr_deserializer::CDRDeserializerAdapter;

#[derive(Debug, Default)]
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
  pub builtin_endpoint_qos: Option<BuiltinEndpointQos>,
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
    Self::default()
  }

  pub fn generate_spdp_participant_data(&self) -> Result<SpdpDiscoveredParticipantData, Error> {
    Ok(SpdpDiscoveredParticipantData {
      updated_time: Utc::now(),
      protocol_version: self
        .protocol_version
        .ok_or_else(|| log_and_err_discovery!("protocol_version missing"))?,
      vendor_id: self
        .vendor_id
        .ok_or_else(|| log_and_err_discovery!("vendor_id missing"))?,
      expects_inline_qos: self.expects_inline_qos.unwrap_or(false),
      participant_guid: self
        .participant_guid
        .ok_or_else(|| log_and_err_discovery!("participant_guid missing"))?,
      metatraffic_unicast_locators: self.metatraffic_unicast_locators.clone(),
      metatraffic_multicast_locators: self.metatraffic_multicast_locators.clone(),
      default_unicast_locators: self.default_unicast_locators.clone(),
      default_multicast_locators: self.default_multicast_locators.clone(),
      available_builtin_endpoints: self
        .available_builtin_endpoints
        .ok_or_else(|| log_and_err_discovery!("available_builtin_endpoints missing"))?,
      lease_duration: self.lease_duration,
      manual_liveliness_count: self.manual_liveliness_count.unwrap_or(0),
      builtin_endpoint_qos: self.builtin_endpoint_qos,
      entity_name: self.entity_name.clone(),
    })
  }

  pub fn generate_spdp_participant_data_key(
    &self,
  ) -> Result<SpdpDiscoveredParticipantDataKey, Error> {
    Ok(SpdpDiscoveredParticipantDataKey(
      self
        .participant_guid
        .ok_or_else(|| log_and_err_discovery!("participant_guid missing"))?,
    ))
  }

  pub fn generate_reader_proxy(&self) -> Option<ReaderProxy> {
    let remote_reader_guid = match self.endpoint_guid {
      Some(g) => g,
      None => {
        warn!(
          "Discovery received ReaderProxy data without GUID: {:?}",
          self
        );
        return None;
      }
    };
    Some(ReaderProxy {
      remote_reader_guid,
      expects_inline_qos: self.expects_inline_qos.unwrap_or(false),
      unicast_locator_list: self.unicast_locator_list.clone(),
      multicast_locator_list: self.multicast_locator_list.clone(),
    })
  }

  pub fn generate_writer_proxy(&self) -> Option<WriterProxy> {
    let remote_writer_guid = match self.endpoint_guid {
      Some(g) => g,
      None => {
        warn!(
          "Discovery received WriterProxy data without GUID: {:?}",
          self
        );
        return None;
      }
    };
    Some(WriterProxy {
      remote_writer_guid,
      unicast_locator_list: self.unicast_locator_list.clone(),
      multicast_locator_list: self.multicast_locator_list.clone(),
      data_max_size_serialized: self.data_max_size_serialized,
    })
  }

  pub fn generate_subscription_topic_data(&self) -> Result<SubscriptionBuiltinTopicData, Error> {
    let qos = QosPolicyBuilder::new();

    let qos = match self.durability {
      Some(d) => qos.durability(d),
      None => qos,
    };

    let qos = match self.deadline {
      Some(dl) => qos.deadline(dl),
      None => qos,
    };

    let qos = match self.latency_budget {
      Some(lb) => qos.latency_budget(lb),
      None => qos,
    };

    let qos = match self.liveliness {
      Some(l) => qos.liveliness(l),
      None => qos,
    };

    let qos = match self.reliability {
      Some(r) => qos.reliability(r),
      None => qos,
    };

    let qos = match self.ownership {
      Some(o) => qos.ownership(o),
      None => qos,
    };

    let qos = match self.destination_order {
      Some(d) => qos.destination_order(d),
      None => qos,
    };

    let qos = match self.time_based_filter {
      Some(tbf) => qos.time_based_filter(tbf),
      None => qos,
    };

    let qos = match self.presentation {
      Some(p) => qos.presentation(p),
      None => qos,
    };

    let qos = match self.lifespan {
      Some(ls) => qos.lifespan(ls),
      None => qos,
    };

    let qos = qos.build();

    let key = match self.endpoint_guid {
      Some(g) => g,
      None => return Err(Error::Message("Failed to parse key.".to_string())),
    };

    let topic_name = match &self.topic_name {
      Some(tn) => tn.clone(),
      None => return Err(Error::Message("Failed to parse topic name.".to_string())),
    };

    let type_name = match &self.type_name {
      Some(tn) => tn.clone(),
      None => return Err(Error::Message("Failed to parse type name.".to_string())),
    };

    let mut sbtd = SubscriptionBuiltinTopicData::new(key, topic_name, type_name, &qos);

    if let Some(g) = self.participant_guid {
      sbtd.set_participant_key(g)
    };

    Ok(sbtd)
  }

  pub fn generate_publication_topic_data(&self) -> Result<PublicationBuiltinTopicData, Error> {
    let key = self
      .endpoint_guid
      .ok_or_else(|| Error::Message("generate_publication_topic_data: No GUID".to_string()))?;

    Ok(PublicationBuiltinTopicData {
      key,
      participant_key: self.participant_guid,
      topic_name: self
        .topic_name
        .clone()
        .ok_or_else(|| Error::Message("Failed to parse topic name.".to_string()))?,
      type_name: self
        .type_name
        .clone()
        .ok_or_else(|| Error::Message("Failed to parse topic type.".to_string()))?,
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
    })
  }

  pub fn generate_topic_data(self) -> Result<TopicBuiltinTopicData, Error> {
    Ok(TopicBuiltinTopicData {
      key: self.endpoint_guid,
      name: self
        .topic_name
        .ok_or_else(|| Error::Message("Failed to parse topic name.".to_string()))?,
      type_name: self
        .type_name
        .ok_or_else(|| Error::Message("Failed to parse topic type.".to_string()))?,
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
    })
  }

  pub fn generate_discovered_reader_data(self) -> Result<DiscoveredReaderData, Error> {
    let reader_proxy = self
      .generate_reader_proxy()
      .ok_or_else(|| Error::Message("ReaderProxy deserialization".to_string()))?;
    let subscription_topic_data = self.generate_subscription_topic_data()?;
    Ok(DiscoveredReaderData {
      reader_proxy,
      subscription_topic_data,
      content_filter: self.content_filter_property,
    })
  }

  pub fn generate_discovered_writer_data(self) -> Result<DiscoveredWriterData, Error> {
    let writer_proxy = self
      .generate_writer_proxy()
      .ok_or_else(|| Error::Message("WriterProxy deserialization".to_string()))?;
    let publication_topic_data = self.generate_publication_topic_data()?;
    Ok(DiscoveredWriterData {
      last_updated: Instant::now(),
      writer_proxy,
      publication_topic_data,
    })
  }

  pub fn generate_discovered_reader_data_key(self) -> Result<DiscoveredReaderDataKey, Error> {
    Ok(DiscoveredReaderDataKey(self.endpoint_guid.ok_or_else(
      || log_and_err_discovery!("generate_discovered_reader_data_key endpoint_guid missing"),
    )?))
  }

  pub fn generate_discovered_writer_data_key(self) -> Result<DiscoveredWriterDataKey, Error> {
    Ok(DiscoveredWriterDataKey(self.endpoint_guid.ok_or_else(
      || log_and_err_discovery!("generate_discovered_writer_data_key endpoint_guid missing"),
    )?))
  }

  pub fn parse_data_little_endian(self, buffer: &[u8]) -> BuiltinDataDeserializer {
    self.parse_data(buffer, RepresentationIdentifier::CDR_LE)
  }

  pub fn parse_data_big_endian(self, buffer: &[u8]) -> BuiltinDataDeserializer {
    self.parse_data(buffer, RepresentationIdentifier::CDR_BE)
  }

  fn parse_data(mut self, buffer: &[u8], rep: RepresentationIdentifier) -> BuiltinDataDeserializer {
    let mut buffer = buffer.to_vec();
    while self.sentinel.is_none() && !buffer.is_empty() {
      self = self.read_next(&mut buffer, rep);
    }

    self
  }

  pub fn read_next(
    mut self,
    buffer: &mut Vec<u8>,
    rep: RepresentationIdentifier,
  ) -> BuiltinDataDeserializer {
    let parameter_id = BuiltinDataDeserializer::read_parameter_id(buffer, rep).unwrap();
    let mut parameter_length: usize =
      BuiltinDataDeserializer::read_parameter_length(buffer, rep).unwrap() as usize;

    if (parameter_length + 4) > buffer.len() {
      parameter_length = buffer.len() - 4;
    }

    match parameter_id {
      ParameterId::PID_PARTICIPANT_GUID => {
        let guid: Result<GUID, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(gg) = guid {
          self.participant_guid = Some(gg);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_PROTOCOL_VERSION => {
        let version: Result<ProtocolVersion, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(vv) = version {
          self.protocol_version = Some(vv);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_VENDOR_ID => {
        let vendor: Result<VendorId, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(vv) = vendor {
          self.vendor_id = Some(vv);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_EXPECTS_INLINE_QOS => {
        let inline_qos: Result<bool, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(qos) = inline_qos {
          self.expects_inline_qos = Some(qos);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_METATRAFFIC_UNICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(loc) = locator {
          self.metatraffic_unicast_locators.push(loc);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_METATRAFFIC_MULTICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(loc) = locator {
          self.metatraffic_multicast_locators.push(loc);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_DEFAULT_UNICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(loc) = locator {
          self.default_unicast_locators.push(loc);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_DEFAULT_MULTICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(loc) = locator {
          self.default_multicast_locators.push(loc);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_BUILTIN_ENDPOINT_SET => {
        let endpoints: Result<BuiltinEndpointSet, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(ep) = endpoints {
          self.available_builtin_endpoints = Some(ep);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_PARTICIPANT_LEASE_DURATION => {
        let duration: Result<Duration, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(dur) = duration {
          self.lease_duration = Some(dur);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT => {
        let count: Result<i32, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(c) = count {
          self.manual_liveliness_count = Some(c);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_BUILTIN_ENDPOINT_QOS => {
        let qos: Result<BuiltinEndpointQos, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(q) = qos {
          self.builtin_endpoint_qos = Some(q);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_ENTITY_NAME => {
        let name: Result<String, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(n) = name {
          self.entity_name = Some(n);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_ENDPOINT_GUID => {
        let guid: Result<GUID, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(gg) = guid {
          self.endpoint_guid = Some(gg);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_UNICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(loc) = locator {
          self.unicast_locator_list.push(loc);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_MULTICAST_LOCATOR => {
        let locator: Result<Locator, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(loc) = locator {
          self.multicast_locator_list.push(loc);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_TOPIC_NAME => {
        let topic_name: Result<String, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(name) = topic_name {
          self.topic_name = Some(name);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_TYPE_NAME => {
        let type_name: Result<String, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(name) = type_name {
          self.type_name = Some(name);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_DURABILITY => {
        let durability: Result<Durability, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(dur) = durability {
          self.durability = Some(dur);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_DEADLINE => {
        let deadline: Result<Deadline, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(dl) = deadline {
          self.deadline = Some(dl);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_LATENCY_BUDGET => {
        let latency_budget: Result<LatencyBudget, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(lb) = latency_budget {
          self.latency_budget = Some(lb);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_LIVELINESS => {
        #[derive(Deserialize, Clone, Copy)]
        #[repr(u32)]
        enum LivelinessKind {
          Automatic,
          ManualByParticipant,
          ManualbyTopic,
        }
        #[derive(Deserialize, Clone, Copy)]
        struct LivelinessData {
          pub kind: LivelinessKind,
          pub lease_duration: Duration,
        }
        let liveliness: Result<LivelinessData, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(liv) = liveliness {
          self.liveliness = match liv.kind {
            LivelinessKind::Automatic => Some(Liveliness::Automatic {
              lease_duration: liv.lease_duration,
            }),
            LivelinessKind::ManualByParticipant => Some(Liveliness::ManualByParticipant {
              lease_duration: liv.lease_duration,
            }),
            LivelinessKind::ManualbyTopic => Some(Liveliness::ManualByTopic {
              lease_duration: liv.lease_duration,
            }),
          };
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_RELIABILITY => {
        #[derive(Deserialize, Clone)]
        struct ReliabilityBestEffortData {
          pub reliability_kind: ReliabilityKind,
          pub max_blocking_time: Duration,
        }

        let reliability: Result<ReliabilityBestEffortData, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(rel) = reliability {
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
      }
      ParameterId::PID_OWNERSHIP => {
        #[derive(Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        enum OwnershipKind {
          Shared,
          Exclusive,
        }
        let ownership: Result<OwnershipKind, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(own) = ownership {
          let strength = match self.ownership {
            Some(Ownership::Exclusive { strength }) => strength,
            _ => 0,
          };

          let own = match own {
            OwnershipKind::Shared => Ownership::Shared,
            OwnershipKind::Exclusive => Ownership::Exclusive { strength },
          };

          self.ownership = Some(own);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_OWNERSHIP_STRENGTH => {
        let ownership_strength: Result<i32, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(stri) = ownership_strength {
          self.ownership = match self.ownership {
            Some(v) => match v {
              Ownership::Exclusive { strength: _ } => Some(Ownership::Exclusive { strength: stri }),
              _ => Some(v),
            },
            None => Some(Ownership::Exclusive { strength: stri }),
          }
        };
        buffer.drain(..4 + parameter_length);
        return self;
      }
      ParameterId::PID_DESTINATION_ORDER => {
        let destination_order: Result<DestinationOrder, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(deor) = destination_order {
          self.destination_order = Some(deor);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_TIME_BASED_FILTER => {
        let time_based_filter: Result<TimeBasedFilter, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(tbf) = time_based_filter {
          self.time_based_filter = Some(tbf);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_PRESENTATION => {
        let presentation: Result<Presentation, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(p) = presentation {
          self.presentation = Some(p);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_LIFESPAN => {
        let lifespan: Result<Lifespan, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(ls) = lifespan {
          self.lifespan = Some(ls);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_CONTENT_FILTER_PROPERTY => {
        let content_filter: Result<ContentFilterProperty, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(cfp) = content_filter {
          self.content_filter_property = Some(cfp);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_TYPE_MAX_SIZE_SERIALIZED => {
        let max_size: Result<u32, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(ms) = max_size {
          self.data_max_size_serialized = Some(ms);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_HISTORY => {
        #[derive(Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        enum HistoryKind {
          KeepLast,
          KeepAll,
        }

        #[derive(Deserialize)]
        struct HistoryData {
          pub kind: HistoryKind,
          pub depth: i32,
        }

        let history: Result<HistoryData, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(his) = history {
          let h = match his.kind {
            HistoryKind::KeepLast => History::KeepLast { depth: his.depth },
            HistoryKind::KeepAll => History::KeepAll,
          };
          self.history = Some(h);
          buffer.drain(..4 + parameter_length);
          return self;
        }
      }
      ParameterId::PID_RESOURCE_LIMITS => {
        let resource_limits: Result<ResourceLimits, Error> =
          CDRDeserializerAdapter::from_bytes(&buffer[4..4 + parameter_length], rep);
        if let Ok(lim) = resource_limits {
          self.resource_limits = Some(lim);
          buffer.drain(..4 + parameter_length);
          return self;
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

  pub fn read_parameter_id(buffer: &[u8], rep: RepresentationIdentifier) -> Option<ParameterId> {
    let par: Result<ParameterId, Error> = CDRDeserializerAdapter::from_bytes(&buffer[..2], rep);
    match par {
      Ok(val) => Some(val),
      _ => None,
    }
  }

  pub fn read_parameter_length(buffer: &[u8], rep: RepresentationIdentifier) -> Option<u16> {
    if buffer.len() < 4 {
      return Some(0);
    }

    let parameter_length_value = CDRDeserializerAdapter::from_bytes(&buffer[2..4], rep);
    match parameter_length_value {
      Ok(val) => Some(val),
      _ => None,
    }
  }
}
