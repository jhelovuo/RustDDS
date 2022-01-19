use std::time::Instant;

use serde::{de, Deserialize, Serialize};
use chrono::Utc;
use cdr_encoding_size::*;

use crate::{
  dds::{
    participant::DomainParticipant,
    qos::{
      policy::{
        Deadline, DestinationOrder, Durability, History, LatencyBudget, Lifespan, Liveliness,
        Ownership, Presentation, Reliability, ResourceLimits, TimeBasedFilter,
      },
      HasQoSPolicy, QosPolicies,
    },
    rtps_reader_proxy::RtpsReaderProxy,
    topic::Topic,
    traits::{
      key::{Key, Keyed},
      serde_adapters::with_key::SerializerAdapter,
      TopicDescription,
    },
    with_key::datawriter::DataWriter,
  },
  discovery::content_filter_property::ContentFilterProperty,
  network::{constant::user_traffic_unicast_port, util::get_local_unicast_locators},
  serialization::{
    builtin_data_deserializer::BuiltinDataDeserializer,
    builtin_data_serializer::BuiltinDataSerializer,
  },
  structure::{
    entity::RTPSEntity,
    guid::{GuidPrefix, GUID},
    locator::Locator,
  },
};
#[cfg(test)]
use crate::structure::guid::EntityKind;

// Topic data contains all topic related
// (including reader and writer data structures for serialization and
// deserialization)

/// Type specified in RTPS v2.3 spec Figure 8.30
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReaderProxy {
  pub remote_reader_guid: GUID,
  pub expects_inline_qos: bool,
  pub unicast_locator_list: Vec<Locator>,
  pub multicast_locator_list: Vec<Locator>,
}

impl ReaderProxy {
  pub fn new(
    guid: GUID,
    unicast_locator_list: Vec<Locator>,
    multicast_locator_list: Vec<Locator>,
  ) -> ReaderProxy {
    ReaderProxy {
      remote_reader_guid: guid,
      expects_inline_qos: false,
      unicast_locator_list,
      multicast_locator_list,
    }
  }
}

impl From<RtpsReaderProxy> for ReaderProxy {
  fn from(rtps_reader_proxy: RtpsReaderProxy) -> Self {
    ReaderProxy {
      remote_reader_guid: rtps_reader_proxy.remote_reader_guid,
      expects_inline_qos: rtps_reader_proxy.expects_in_line_qos,
      unicast_locator_list: rtps_reader_proxy.unicast_locator_list,
      multicast_locator_list: rtps_reader_proxy.multicast_locator_list,
    }
  }
}

impl<'de> Deserialize<'de> for ReaderProxy {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    res
      .generate_reader_proxy()
      .ok_or_else(|| de::Error::custom("proxy desrialization"))
  }
}

impl Serialize for ReaderProxy {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_reader_proxy(self);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

// =======================================================================
// =======================================================================
// =======================================================================

/// DDS SubscriptionBuiltinTopicData
/// Type specified in RTPS v2.3 spec Figure 8.30
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriptionBuiltinTopicData {
  key: GUID,
  participant_key: Option<GUID>,
  topic_name: String,
  type_name: String,
  durability: Option<Durability>,
  deadline: Option<Deadline>,
  latency_budget: Option<LatencyBudget>,
  liveliness: Option<Liveliness>,
  reliability: Option<Reliability>,
  ownership: Option<Ownership>,
  destination_order: Option<DestinationOrder>,
  // pub user_data: Option<UserData>,
  time_based_filter: Option<TimeBasedFilter>,
  presentation: Option<Presentation>,
  // pub partition: Option<Partition>,
  // pub topic_data: Option<TopicData>,
  // pub group_data: Option<GroupData>,
  // pub durability_service: Option<DurabilityService>,
  lifespan: Option<Lifespan>,

  // From spec Remote Procedure Call over DDS:
  service_instance_name: Option<String>,
  related_datawriter_key: Option<GUID>,
  topic_aliases: Option<Vec<String>>, /* Option is a bit redundant, but it indicates if the
                                       * parameter was present or not */
}

impl SubscriptionBuiltinTopicData {
  pub fn new(
    key: GUID,
    topic_name: String,
    type_name: String,
    qos: &QosPolicies,
  ) -> SubscriptionBuiltinTopicData {
    let mut sbtd = SubscriptionBuiltinTopicData {
      key,
      participant_key: None,
      topic_name,
      type_name,
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
      service_instance_name: None,
      related_datawriter_key: None,
      topic_aliases: None,
    };

    sbtd.set_qos(qos);
    sbtd
  }

  pub fn key(&self) -> GUID {
    self.key
  }

  pub fn set_key(&mut self, key: GUID) {
    self.key = key;
  }

  pub fn participant_key(&self) -> &Option<GUID> {
    &self.participant_key
  }

  pub fn set_participant_key(&mut self, participant_key: GUID) {
    self.participant_key = Some(participant_key);
  }

  pub fn topic_name(&self) -> &String {
    &self.topic_name
  }

  pub fn set_topic_name(&mut self, topic_name: &str) {
    self.topic_name = String::from(topic_name);
  }

  pub fn type_name(&self) -> &String {
    &self.type_name
  }

  pub fn set_type_name(&mut self, type_name: &str) {
    self.type_name = String::from(type_name);
  }

  pub fn durability(&self) -> &Option<Durability> {
    &self.durability
  }

  pub fn deadline(&self) -> &Option<Deadline> {
    &self.deadline
  }

  pub fn latency_budget(&self) -> &Option<LatencyBudget> {
    &self.latency_budget
  }

  pub fn liveliness(&self) -> &Option<Liveliness> {
    &self.liveliness
  }

  pub fn reliability(&self) -> &Option<Reliability> {
    &self.reliability
  }

  pub fn ownership(&self) -> &Option<Ownership> {
    &self.ownership
  }

  pub fn destination_order(&self) -> &Option<DestinationOrder> {
    &self.destination_order
  }

  pub fn time_based_filter(&self) -> &Option<TimeBasedFilter> {
    &self.time_based_filter
  }

  pub fn presentation(&self) -> &Option<Presentation> {
    &self.presentation
  }

  pub fn lifespan(&self) -> &Option<Lifespan> {
    &self.lifespan
  }

  pub fn set_qos(&mut self, qos: &QosPolicies) {
    self.durability = qos.durability;
    self.deadline = qos.deadline;
    self.latency_budget = qos.latency_budget;
    self.liveliness = qos.liveliness;
    self.reliability = qos.reliability;
    self.ownership = qos.ownership;
    self.destination_order = qos.destination_order;
    self.time_based_filter = qos.time_based_filter;
    self.presentation = qos.presentation;
    self.lifespan = qos.lifespan;
  }

  pub fn generate_qos(&self) -> QosPolicies {
    QosPolicies {
      durability: self.durability,
      presentation: self.presentation,
      deadline: self.deadline,
      latency_budget: self.latency_budget,
      ownership: self.ownership,
      liveliness: self.liveliness,
      time_based_filter: self.time_based_filter,
      reliability: self.reliability,
      destination_order: self.destination_order,
      history: None,         // TODO: Check that this really does not exist in source
      resource_limits: None, // TODO: Check that this really does not exist in source
      lifespan: self.lifespan,
    }
  }
}

impl<'de> Deserialize<'de> for SubscriptionBuiltinTopicData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    res
      .generate_subscription_topic_data()
      .map_err(serde::de::Error::custom)
  }
}

impl Serialize for SubscriptionBuiltinTopicData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_subscription_topic_data(self);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}
// ------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriptionBuiltinTopicDataKey(pub GUID);

impl Serialize for SubscriptionBuiltinTopicDataKey {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_endpoint_guid(&self.0);
    builtin_data_serializer.serialize_key::<S>(serializer, false)
  }
}

// =======================================================================
// =======================================================================
// =======================================================================

/// Type specified in RTPS v2.3 spec Figure 8.30
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscoveredReaderData {
  pub reader_proxy: ReaderProxy,
  pub subscription_topic_data: SubscriptionBuiltinTopicData,
  pub content_filter: Option<ContentFilterProperty>,
}

impl DiscoveredReaderData {
  /*
  pub(crate) fn new(
    reader: &Reader,
    dp: &DomainParticipant,
    topic: &Topic,
  ) -> DiscoveredReaderData {
    let reader_proxy = ReaderProxy::new(reader.guid());
    let mut subscription_topic_data = SubscriptionBuiltinTopicData::new(
      reader.guid(),
      topic.name(),
      topic.get_type().name().to_string(),
      &topic.qos(),
    );
    subscription_topic_data.set_participant_key(dp.guid());

    DiscoveredReaderData {
      reader_proxy,
      subscription_topic_data,
      content_filter: None,
    }
  }*/

  // This is for generating test data only
  #[cfg(test)]
  pub fn default(topic_name: String, type_name: String) -> DiscoveredReaderData {
    let rguid = GUID::dummy_test_guid(EntityKind::READER_WITH_KEY_BUILT_IN);
    let reader_proxy = ReaderProxy::new(rguid, vec![], vec![]);
    let subscription_topic_data = SubscriptionBuiltinTopicData::new(
      rguid,
      topic_name,
      type_name,
      &QosPolicies::builder().build(),
    );
    DiscoveredReaderData {
      reader_proxy,
      subscription_topic_data,
      content_filter: None,
    }
  }

  // pub(crate) fn update(&mut self, rtps_reader_proxy: &RtpsReaderProxy) {
  //   self.reader_proxy.remote_reader_guid =
  // rtps_reader_proxy.remote_reader_guid;   self.reader_proxy.
  // expects_inline_qos = rtps_reader_proxy.expects_in_line_qos;
  //   self.reader_proxy.unicast_locator_list =
  // rtps_reader_proxy.unicast_locator_list.clone();   self.reader_proxy.
  // multicast_locator_list = rtps_reader_proxy.multicast_locator_list.clone();
  // }
}

// separate type is needed to serialize correctly
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone, Copy, Hash, CdrEncodingSize)]
pub struct DiscoveredReaderDataKey(pub GUID);

impl Key for DiscoveredReaderDataKey {}

impl Keyed for DiscoveredReaderData {
  type K = DiscoveredReaderDataKey;
  fn key(&self) -> Self::K {
    DiscoveredReaderDataKey(self.subscription_topic_data.key)
  }
}

impl<'de> Deserialize<'de> for DiscoveredReaderData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    res
      .generate_discovered_reader_data()
      .map_err(serde::de::Error::custom)
  }
}

impl Serialize for DiscoveredReaderData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_discovered_reader_data(self);
    builtin_data_serializer.serialize::<S>(serializer, true)
  }
}

// -------

impl<'de> Deserialize<'de> for DiscoveredReaderDataKey {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    res
      .generate_discovered_reader_data_key()
      .map_err(serde::de::Error::custom)
  }
}

impl Serialize for DiscoveredReaderDataKey {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_discovered_reader_data_key(self);
    builtin_data_serializer.serialize_key::<S>(serializer, true)
  }
}

// =======================================================================
// =======================================================================
// =======================================================================

/// Type specified in RTPS v2.3 spec Figure 8.30
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WriterProxy {
  pub remote_writer_guid: GUID,
  pub unicast_locator_list: Vec<Locator>,
  pub multicast_locator_list: Vec<Locator>,
  pub data_max_size_serialized: Option<u32>,
}

impl WriterProxy {
  pub fn new(
    guid: GUID,
    multicast_locator_list: Vec<Locator>,
    unicast_locator_list: Vec<Locator>,
  ) -> WriterProxy {
    WriterProxy {
      remote_writer_guid: guid,
      unicast_locator_list,
      multicast_locator_list,
      data_max_size_serialized: None,
    }
  }
}

impl<'de> Deserialize<'de> for WriterProxy {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    res
      .generate_writer_proxy()
      .ok_or_else(|| de::Error::custom("WriterProxy deserialization"))
  }
}

impl Serialize for WriterProxy {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_writer_proxy(self);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

// =======================================================================
// =======================================================================
// =======================================================================

/// Type specified in RTPS v2.3 spec Figure 8.30
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicationBuiltinTopicData {
  pub key: GUID, // endpoint GUID
  pub participant_key: Option<GUID>,
  pub topic_name: String,
  pub type_name: String,
  pub durability: Option<Durability>,
  pub deadline: Option<Deadline>,
  pub latency_budget: Option<LatencyBudget>,
  pub liveliness: Option<Liveliness>,
  pub reliability: Option<Reliability>,
  pub lifespan: Option<Lifespan>,
  pub time_based_filter: Option<TimeBasedFilter>,
  pub ownership: Option<Ownership>,
  pub destination_order: Option<DestinationOrder>,
  pub presentation: Option<Presentation>,

  // From Remote Procedure Call over DDS:
  pub service_instance_name: Option<String>,
  pub related_datareader_key: Option<GUID>,
  pub topic_aliases: Option<Vec<String>>, /* Option is a bit redundant, but it indicates
                                           * if the parameter was present or not */
}

impl PublicationBuiltinTopicData {
  pub fn new(
    guid: GUID,
    participant_guid: GUID,
    topic_name: String,
    type_name: String,
  ) -> PublicationBuiltinTopicData {
    PublicationBuiltinTopicData {
      key: guid,
      participant_key: Some(participant_guid),
      topic_name,
      type_name,
      durability: None,
      deadline: None,
      latency_budget: None,
      liveliness: None,
      reliability: None,
      lifespan: None,
      time_based_filter: None,
      ownership: None,
      destination_order: None,
      presentation: None,

      service_instance_name: None,
      related_datareader_key: None,
      topic_aliases: None,
    }
  }

  pub fn read_qos(&mut self, qos: &QosPolicies) {
    self.durability = qos.durability;
    self.deadline = qos.deadline;
    self.latency_budget = qos.latency_budget;
    self.liveliness = qos.liveliness;
    self.reliability = qos.reliability;
    self.lifespan = qos.lifespan;
    self.time_based_filter = qos.time_based_filter;
    self.ownership = qos.ownership;
    self.destination_order = qos.destination_order;
    self.presentation = qos.presentation;
  }

  pub fn qos(&self) -> QosPolicies {
    QosPolicies {
      durability: self.durability,
      presentation: self.presentation,
      deadline: self.deadline,
      latency_budget: self.latency_budget,
      ownership: self.ownership,
      liveliness: self.liveliness,
      time_based_filter: self.time_based_filter,
      reliability: self.reliability,
      destination_order: self.destination_order,
      history: None,         // TODO: ???
      resource_limits: None, // TODO: ???
      lifespan: self.lifespan,
    }
  }
}

impl<'de> Deserialize<'de> for PublicationBuiltinTopicData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    res
      .generate_publication_topic_data()
      .map_err(de::Error::custom)
  }
}

impl Serialize for PublicationBuiltinTopicData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_publication_topic_data(self);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

// ------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, CdrEncodingSize)]
pub struct PublicationBuiltinTopicDataKey(pub GUID);

impl Serialize for PublicationBuiltinTopicDataKey {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_endpoint_guid(&self.0);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

// =======================================================================
// =======================================================================
// =======================================================================

/// Type specified in RTPS v2.3 spec Figure 8.30
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscoveredWriterData {
  // last_updated is not serialized
  pub last_updated: Instant,
  pub writer_proxy: WriterProxy,
  pub publication_topic_data: PublicationBuiltinTopicData,
}

// separate type is needed to serialize correctly
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone, Copy, Hash, CdrEncodingSize)]
pub struct DiscoveredWriterDataKey(pub GUID); // wrapper to enable custom PL CDR (de)serialization

impl Key for DiscoveredWriterDataKey {}

impl Keyed for DiscoveredWriterData {
  type K = DiscoveredWriterDataKey;

  fn key(&self) -> Self::K {
    DiscoveredWriterDataKey(self.publication_topic_data.key)
  }
}

impl DiscoveredWriterData {
  pub fn new<D: Keyed + Serialize, SA: SerializerAdapter<D>>(
    writer: &DataWriter<D, SA>,
    topic: &Topic,
    dp: &DomainParticipant,
  ) -> DiscoveredWriterData {
    let unicast_port = user_traffic_unicast_port(dp.domain_id(), dp.participant_id());
    let unicast_addresses = get_local_unicast_locators(unicast_port);

    let writer_proxy = WriterProxy::new(writer.guid(), vec![], unicast_addresses);
    let mut publication_topic_data = PublicationBuiltinTopicData::new(
      writer.guid(),
      dp.guid(),
      topic.name(),
      topic.get_type().name().to_string(),
    );

    publication_topic_data.read_qos(&topic.qos());

    DiscoveredWriterData {
      last_updated: Instant::now(),
      writer_proxy,
      publication_topic_data,
    }
  }

  // pub(crate) fn update(&mut self, rtps_writer_proxy: &RtpsWriterProxy) {
  //   self.writer_proxy.remote_writer_guid =
  // rtps_writer_proxy.remote_writer_guid;   self.writer_proxy.
  // unicast_locator_list = rtps_writer_proxy.unicast_locator_list.clone();
  //   self.writer_proxy.multicast_locator_list =
  // rtps_writer_proxy.multicast_locator_list.clone(); }
}

impl<'de> Deserialize<'de> for DiscoveredWriterData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    res
      .generate_discovered_writer_data()
      .map_err(de::Error::custom)
  }
}

impl Serialize for DiscoveredWriterData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_discovered_writer_data(self);
    builtin_data_serializer.serialize::<S>(serializer, true)
  }
}

impl<'de> Deserialize<'de> for DiscoveredWriterDataKey {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    res
      .generate_discovered_writer_data_key()
      .map_err(de::Error::custom)
  }
}

impl Serialize for DiscoveredWriterDataKey {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_discovered_writer_data_key(self);
    builtin_data_serializer.serialize_key::<S>(serializer, true)
  }
}

// =======================================================================
// =======================================================================
// =======================================================================

/// Type specified in RTPS v2.3 spec Figure 8.30
#[derive(Debug, PartialEq, Clone)]
pub struct TopicBuiltinTopicData {
  pub key: Option<GUID>,
  pub name: String,
  pub type_name: String,
  pub durability: Option<Durability>,
  pub deadline: Option<Deadline>,
  pub latency_budget: Option<LatencyBudget>,
  pub liveliness: Option<Liveliness>,
  pub reliability: Option<Reliability>,
  pub lifespan: Option<Lifespan>,
  pub destination_order: Option<DestinationOrder>,
  pub presentation: Option<Presentation>,
  pub history: Option<History>,
  pub resource_limits: Option<ResourceLimits>,
  pub ownership: Option<Ownership>,
}

impl HasQoSPolicy for TopicBuiltinTopicData {
  fn qos(&self) -> QosPolicies {
    QosPolicies {
      durability: self.durability,
      presentation: self.presentation,
      deadline: self.deadline,
      latency_budget: self.latency_budget,
      ownership: self.ownership,
      liveliness: self.liveliness,
      time_based_filter: None,
      reliability: self.reliability,
      destination_order: self.destination_order,
      history: self.history,
      resource_limits: self.resource_limits,
      lifespan: self.lifespan,
    }
  }
}

impl<'de> Deserialize<'de> for TopicBuiltinTopicData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    res.generate_topic_data().map_err(de::Error::custom)
  }
}

impl Serialize for TopicBuiltinTopicData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_topic_data(self);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

// =======================================================================
// =======================================================================
// =======================================================================

/// DDS Spec defined DiscoveredTopicData with extra updated time attribute.
/// Practically this is gotten from
/// [DomainParticipant](../participant/struct.DomainParticipant.html) during
/// runtime Type specified in RTPS v2.3 spec Figure 8.30
#[derive(Debug, PartialEq, Clone)]
pub struct DiscoveredTopicData {
  pub updated_time: u64,
  pub topic_data: TopicBuiltinTopicData,
}

impl DiscoveredTopicData {
  pub fn new(topic_data: TopicBuiltinTopicData) -> DiscoveredTopicData {
    DiscoveredTopicData {
      updated_time: Utc::now().timestamp_nanos() as u64,
      topic_data,
    }
  }

  pub fn topic_name(&self) -> &String {
    &self.topic_data.name
  }

  pub fn type_name(&self) -> &String {
    &self.topic_data.type_name
  }
}

impl<'de> Deserialize<'de> for DiscoveredTopicData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    let topic_data = res.generate_topic_data().map_err(de::Error::custom)?;

    Ok(DiscoveredTopicData::new(topic_data))
  }
}

impl Serialize for DiscoveredTopicData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_topic_data(&self.topic_data);
    builtin_data_serializer.serialize::<S>(serializer, true)
  }
}

pub type DiscoveredTopicDataKey = GUID;

impl Keyed for DiscoveredTopicData {
  type K = GUID;

  fn key(&self) -> Self::K {
    // topic should always have a name, if this crashes the problem is in the
    // overall logic (or message parsing)
    match self.topic_data.key {
      Some(k) => k,
      None => GUID::GUID_UNKNOWN,
    }
  }
}

// =======================================================================
// =======================================================================
// =======================================================================

#[derive(
  Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize, CdrEncodingSize,
)]
pub struct ParticipantMessageDataKind {
  value: [u8; 4],
}

impl ParticipantMessageDataKind {
  #[allow(dead_code)] // This is defined in the spec, but currenty unused.
  pub const UNKNOWN: ParticipantMessageDataKind = ParticipantMessageDataKind {
    value: [0x00, 0x00, 0x00, 0x00],
  };
  pub const AUTOMATIC_LIVELINESS_UPDATE: ParticipantMessageDataKind = ParticipantMessageDataKind {
    value: [0x00, 0x00, 0x00, 0x01],
  };
  pub const MANUAL_LIVELINESS_UPDATE: ParticipantMessageDataKind = ParticipantMessageDataKind {
    value: [0x00, 0x00, 0x00, 0x02],
  };
}

// =======================================================================
// =======================================================================
// =======================================================================

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ParticipantMessageData {
  pub guid: GuidPrefix,
  pub kind: ParticipantMessageDataKind,
  // normally this should be empty
  // pub length: u32, // encoding the length is implicit in the CDR encoding of Vec
  pub data: Vec<u8>,
}

impl Keyed for ParticipantMessageData {
  type K = ParticipantMessageDataKey;

  fn key(&self) -> Self::K {
    ParticipantMessageDataKey(self.guid, self.kind)
  }
}

#[derive(
  Debug, PartialEq, Eq, Clone, Copy, Hash, Ord, PartialOrd, Serialize, Deserialize, CdrEncodingSize,
)]
pub struct ParticipantMessageDataKey(GuidPrefix, ParticipantMessageDataKind);

impl Key for ParticipantMessageDataKey {}

// =======================================================================
// =======================================================================
// =======================================================================

#[cfg(test)]
mod tests {
  use byteorder::LittleEndian;
  use bytes::Bytes;
  use log::info;

  use super::*;
  // use crate::serialization::cdr_serializer::to_little_endian_binary;
  use crate::{
    dds::traits::serde_adapters::no_key::DeserializerAdapter,
    messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier,
    serialization::{
      cdr_serializer::to_bytes, pl_cdr_deserializer::PlCdrDeserializerAdapter, Message,
    },
    test::test_data::{
      content_filter_data, publication_builtin_topic_data, reader_proxy_data,
      subscription_builtin_topic_data, topic_data, writer_proxy_data,
    },
  };

  #[test]
  fn td_reader_proxy_ser_deser() {
    let reader_proxy = reader_proxy_data().unwrap();

    let sdata = to_bytes::<ReaderProxy, LittleEndian>(&reader_proxy).unwrap();
    let reader_proxy2: ReaderProxy =
      PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE).unwrap();
    assert_eq!(reader_proxy, reader_proxy2);
    let sdata2 = to_bytes::<ReaderProxy, LittleEndian>(&reader_proxy2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_writer_proxy_ser_deser() {
    let writer_proxy = writer_proxy_data().unwrap();

    let sdata = to_bytes::<WriterProxy, LittleEndian>(&writer_proxy).unwrap();
    let writer_proxy2: WriterProxy =
      PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE).unwrap();
    assert_eq!(writer_proxy, writer_proxy2);
    let sdata2 = to_bytes::<WriterProxy, LittleEndian>(&writer_proxy2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_subscription_builtin_topic_data_ser_deser() {
    let sub_topic_data = subscription_builtin_topic_data().unwrap();

    let sdata = to_bytes::<SubscriptionBuiltinTopicData, LittleEndian>(&sub_topic_data).unwrap();
    let sub_topic_data2: SubscriptionBuiltinTopicData =
      PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE).unwrap();
    assert_eq!(sub_topic_data, sub_topic_data2);
    let sdata2 = to_bytes::<SubscriptionBuiltinTopicData, LittleEndian>(&sub_topic_data2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_publication_builtin_topic_data_ser_deser() {
    let pub_topic_data = publication_builtin_topic_data().unwrap();

    let sdata = to_bytes::<PublicationBuiltinTopicData, LittleEndian>(&pub_topic_data).unwrap();
    let pub_topic_data2: PublicationBuiltinTopicData =
      PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE).unwrap();
    assert_eq!(pub_topic_data, pub_topic_data2);
    let sdata2 = to_bytes::<PublicationBuiltinTopicData, LittleEndian>(&pub_topic_data2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_discovered_reader_data_ser_deser() {
    let mut reader_proxy = reader_proxy_data().unwrap();
    let sub_topic_data = subscription_builtin_topic_data().unwrap();
    reader_proxy.remote_reader_guid = sub_topic_data.key;
    let content_filter = content_filter_data().unwrap();

    let drd = DiscoveredReaderData {
      reader_proxy,
      subscription_topic_data: sub_topic_data,
      content_filter: Some(content_filter),
    };

    let sdata = to_bytes::<DiscoveredReaderData, LittleEndian>(&drd).unwrap();
    let drd2: DiscoveredReaderData =
      PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE).unwrap();
    assert_eq!(drd, drd2);
    let sdata2 = to_bytes::<DiscoveredReaderData, LittleEndian>(&drd2).unwrap();
    assert_eq!(sdata, sdata2);

    let raw_data = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x00, 0x00, 0x39, 0xbc, 0xd6, 0xb1, 0x4f, 0xa2, 0x49,
      0x72, 0x81, 0x7d, 0xd4, 0x54, 0x09, 0x01, 0x08, 0x00, 0xa8, 0x3b, 0x56, 0x5f, 0xfa, 0xa6,
      0xa9, 0x75, 0x15, 0x05, 0xdc, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x04, 0xc7, 0x00,
      0x00, 0x04, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
      0x43, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x00, 0x10, 0x00, 0x39, 0xbc, 0xd6,
      0xb1, 0x4f, 0xa2, 0x49, 0x72, 0x81, 0x7d, 0xd4, 0x54, 0x00, 0x00, 0x01, 0xc1, 0x5a, 0x00,
      0x10, 0x00, 0x39, 0xbc, 0xd6, 0xb1, 0x4f, 0xa2, 0x49, 0x72, 0x81, 0x7d, 0xd4, 0x54, 0x4f,
      0xe7, 0xe7, 0x07, 0x2f, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xfd, 0x1c, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x50, 0x8e,
      0xc9, 0x2f, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xfd, 0x1c, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0xa8, 0x45, 0x14, 0x2f,
      0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xfd, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xac, 0x11, 0x00, 0x01, 0x30, 0x00, 0x18,
      0x00, 0x01, 0x00, 0x00, 0x00, 0xe9, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xef, 0xff, 0x00, 0x01, 0x05, 0x00, 0x0c, 0x00, 0x07,
      0x00, 0x00, 0x00, 0x53, 0x71, 0x75, 0x61, 0x72, 0x65, 0x00, 0x00, 0x07, 0x00, 0x0c, 0x00,
      0x07, 0x00, 0x00, 0x00, 0x53, 0x71, 0x75, 0x61, 0x72, 0x65, 0x00, 0x00, 0x01, 0x00, 0x00,
      0x00,
    ]);

    let msg = Message::read_from_buffer(&raw_data).unwrap();
    info!("{:?}", msg);
  }

  #[test]
  fn td_discovered_writer_data_ser_deser() {
    let mut writer_proxy = writer_proxy_data().unwrap();
    let pub_topic_data = publication_builtin_topic_data().unwrap();
    writer_proxy.remote_writer_guid = pub_topic_data.key;

    let dwd = DiscoveredWriterData {
      last_updated: Instant::now(),
      writer_proxy,
      publication_topic_data: pub_topic_data,
    };

    let sdata = to_bytes::<DiscoveredWriterData, LittleEndian>(&dwd).unwrap();
    let mut dwd2: DiscoveredWriterData =
      PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE).unwrap();
    // last updated is not serialized thus copying value for correct result
    dwd2.last_updated = dwd.last_updated;

    assert_eq!(dwd, dwd2);
    let sdata2 = to_bytes::<DiscoveredWriterData, LittleEndian>(&dwd2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_topic_data_ser_deser() {
    let topic_data = topic_data().unwrap();

    let sdata = to_bytes::<TopicBuiltinTopicData, LittleEndian>(&topic_data).unwrap();
    let topic_data2: TopicBuiltinTopicData =
      PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE).unwrap();
    assert_eq!(topic_data, topic_data2);
    let sdata2 = to_bytes::<TopicBuiltinTopicData, LittleEndian>(&topic_data2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_discovered_topic_data_ser_deser() {
    let topic_data = topic_data().unwrap();

    let dtd = DiscoveredTopicData::new(topic_data);

    let sdata = to_bytes::<DiscoveredTopicData, LittleEndian>(&dtd).unwrap();
    let dtd2: DiscoveredTopicData =
      PlCdrDeserializerAdapter::from_bytes(&sdata, RepresentationIdentifier::PL_CDR_LE).unwrap();
    assert_eq!(dtd.topic_data, dtd2.topic_data);
    let sdata2 = to_bytes::<DiscoveredTopicData, LittleEndian>(&dtd2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  // TODO: somehow get some actual bytes of ParticipantMessageData
  // #[test]
  // fn td_participant_message_data_ser_deser() {
  //   let mut pmd_file = File::open("participant_message_data.bin").unwrap();
  //   let mut buffer: [u8; 1024] = [0; 1024];
  //   let _len = pmd_file.read(&mut buffer).unwrap();

  //   let rpi = CDRDeserializerAdapter::<ParticipantMessageData>::from_bytes(
  //     &buffer,
  //     RepresentationIdentifier::CDR_LE,
  //   )
  //   .unwrap();

  //   let _data2 = to_bytes::<ParticipantMessageData,
  // LittleEndian>(&rpi).unwrap(); }
}
