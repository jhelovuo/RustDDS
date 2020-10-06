use std::time::Instant;

use serde::{Serialize, Deserialize};

use crate::{
  dds::{
    qos::policy::{
      Deadline, Durability, LatencyBudget, Reliability, Ownership, DestinationOrder, Liveliness,
      TimeBasedFilter, Presentation, Lifespan, History, ResourceLimits,
    },
    traits::key::Keyed,
    traits::serde_adapters::SerializerAdapter,
    rtps_reader_proxy::RtpsReaderProxy,
    reader::Reader,
    participant::DomainParticipant,
    topic::Topic,
    datawriter::DataWriter,
    rtps_writer_proxy::RtpsWriterProxy,
  },
  dds::qos::HasQoSPolicy,
  dds::qos::QosPolicies,
  dds::traits::key::Key,
  discovery::content_filter_property::ContentFilterProperty,
  network::constant::get_user_traffic_unicast_port,
  network::util::get_local_unicast_socket_address,
  serialization::{
    builtin_data_serializer::BuiltinDataSerializer,
    builtin_data_deserializer::BuiltinDataDeserializer,
  },
  structure::{entity::Entity, guid::GUID, guid::GuidPrefix, locator::LocatorList},
};

// Topic data contains all topic related (including reader and writer data structures for serialization and deserialization)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReaderProxy {
  pub remote_reader_guid: Option<GUID>,
  pub expects_inline_qos: Option<bool>,
  pub unicast_locator_list: LocatorList,
  pub multicast_locator_list: LocatorList,
}

impl ReaderProxy {
  pub fn new(guid: GUID) -> ReaderProxy {
    ReaderProxy {
      remote_reader_guid: Some(guid),
      expects_inline_qos: Some(false),
      unicast_locator_list: Vec::new(),
      multicast_locator_list: Vec::new(),
    }
  }
}

impl From<RtpsReaderProxy> for ReaderProxy {
  fn from(rtps_reader_proxy: RtpsReaderProxy) -> Self {
    ReaderProxy {
      remote_reader_guid: Some(rtps_reader_proxy.remote_reader_guid),
      expects_inline_qos: Some(rtps_reader_proxy.expects_in_line_qos),
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
    Ok(res.generate_reader_proxy())
  }
}

impl Serialize for ReaderProxy {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_reader_proxy(&self);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriptionBuiltinTopicData {
  pub key: Option<GUID>,
  pub participant_key: Option<GUID>,
  pub topic_name: Option<String>,
  pub type_name: Option<String>,
  pub durability: Option<Durability>,
  pub deadline: Option<Deadline>,
  pub latency_budget: Option<LatencyBudget>,
  pub liveliness: Option<Liveliness>,
  pub reliability: Option<Reliability>,
  pub ownership: Option<Ownership>,
  pub destination_order: Option<DestinationOrder>,
  // pub user_data: Option<UserData>,
  pub time_based_filter: Option<TimeBasedFilter>,
  pub presentation: Option<Presentation>,
  // pub partition: Option<Partition>,
  // pub topic_data: Option<TopicData>,
  // pub group_data: Option<GroupData>,
  // pub durability_service: Option<DurabilityService>,
  pub lifespan: Option<Lifespan>,
}

impl SubscriptionBuiltinTopicData {
  pub fn new(
    key: GUID,
    participant_key: GUID,
    topic_name: &String,
    type_name: &String,
  ) -> SubscriptionBuiltinTopicData {
    SubscriptionBuiltinTopicData {
      key: Some(key),
      participant_key: Some(participant_key),
      topic_name: Some(topic_name.clone()),
      type_name: Some(type_name.clone()),
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
    Ok(res.generate_subscription_topic_data())
  }
}

impl Serialize for SubscriptionBuiltinTopicData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_subscription_topic_data(&self);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscoveredReaderData {
  pub reader_proxy: ReaderProxy,
  pub subscription_topic_data: SubscriptionBuiltinTopicData,
  pub content_filter: Option<ContentFilterProperty>,
}

impl DiscoveredReaderData {
  pub fn new(reader: &Reader, dp: &DomainParticipant, topic: &Topic) -> DiscoveredReaderData {
    let reader_proxy = ReaderProxy::new(reader.get_guid());
    let subscription_topic_data = SubscriptionBuiltinTopicData::new(
      reader.get_guid(),
      dp.get_guid(),
      &topic.get_name().to_string(),
      &topic.get_type().name().to_string(),
    );
    DiscoveredReaderData {
      reader_proxy,
      subscription_topic_data,
      content_filter: None,
    }
  }

  pub fn default(topic_name: &String, type_name: &String) -> DiscoveredReaderData {
    let rguid = GUID::new();
    let reader_proxy = ReaderProxy::new(rguid);
    let mut pguid = GUID::new();
    pguid.guidPrefix = rguid.guidPrefix.clone();
    let subscription_topic_data =
      SubscriptionBuiltinTopicData::new(rguid, pguid, topic_name, type_name);
    DiscoveredReaderData {
      reader_proxy,
      subscription_topic_data,
      content_filter: None,
    }
  }

  pub fn update(&mut self, rtps_reader_proxy: &RtpsReaderProxy) {
    self.reader_proxy.remote_reader_guid = Some(rtps_reader_proxy.remote_reader_guid.clone());
    self.reader_proxy.expects_inline_qos = Some(rtps_reader_proxy.expects_in_line_qos.clone());
    self.reader_proxy.unicast_locator_list = rtps_reader_proxy.unicast_locator_list.clone();
    self.reader_proxy.multicast_locator_list = rtps_reader_proxy.multicast_locator_list.clone();
  }
}

impl Keyed for DiscoveredReaderData {
  type K = GUID;
  fn get_key(&self) -> Self::K {
    match self.subscription_topic_data.key {
      Some(k) => k,
      None => GUID::default(),
    }
  }
}

impl<'de> Deserialize<'de> for DiscoveredReaderData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    Ok(res.generate_discovered_reader_data())
  }
}

//impl DeserializeOwned for DiscoveredReaderData { /*marker trait only */ }

impl Serialize for DiscoveredReaderData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_discovered_reader_data(&self);
    builtin_data_serializer.serialize::<S>(serializer, true)
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WriterProxy {
  pub remote_writer_guid: Option<GUID>,
  pub unicast_locator_list: LocatorList,
  pub multicast_locator_list: LocatorList,
  pub data_max_size_serialized: Option<u32>,
}

impl WriterProxy {
  pub fn new(
    guid: GUID,
    multicast_locator_list: LocatorList,
    unicast_locator_list: LocatorList,
  ) -> WriterProxy {
    WriterProxy {
      remote_writer_guid: Some(guid),
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
    Ok(res.generate_writer_proxy())
  }
}

impl Serialize for WriterProxy {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_writer_proxy(&self);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicationBuiltinTopicData {
  pub key: Option<GUID>,
  pub participant_key: Option<GUID>,
  pub topic_name: Option<String>,
  pub type_name: Option<String>,
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
}

impl PublicationBuiltinTopicData {
  pub fn new(
    guid: GUID,
    participant_guid: GUID,
    topic_name: &String,
    type_name: &String,
  ) -> PublicationBuiltinTopicData {
    PublicationBuiltinTopicData {
      key: Some(guid),
      participant_key: Some(participant_guid),
      topic_name: Some(topic_name.clone()),
      type_name: Some(type_name.clone()),
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
}

impl<'de> Deserialize<'de> for PublicationBuiltinTopicData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    Ok(res.generate_publication_topic_data())
  }
}

impl Serialize for PublicationBuiltinTopicData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_publication_topic_data(&self);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscoveredWriterData {
  // last_updated is not serialized
  pub last_updated: Instant,
  pub writer_proxy: WriterProxy,
  pub publication_topic_data: PublicationBuiltinTopicData,
}

impl Keyed for DiscoveredWriterData {
  type K = GUID;

  fn get_key(&self) -> Self::K {
    match self.publication_topic_data.key {
      Some(k) => k,
      None => GUID::GUID_UNKNOWN,
    }
  }
}

impl DiscoveredWriterData {
  pub fn new<D: Keyed + Serialize, SA: SerializerAdapter<D>>(
    writer: &DataWriter<D, SA>,
    topic: &Topic,
    dp: &DomainParticipant,
  ) -> DiscoveredWriterData {
    let unicast_port = get_user_traffic_unicast_port(dp.domain_id(), dp.participant_id());
    let unicast_addresses = get_local_unicast_socket_address(unicast_port);

    let writer_proxy = WriterProxy::new(writer.get_guid(), vec![], unicast_addresses);
    let mut publication_topic_data = PublicationBuiltinTopicData::new(
      writer.get_guid(),
      dp.get_guid(),
      &topic.get_name().to_string(),
      &topic.get_type().name().to_string(),
    );

    publication_topic_data.read_qos(topic.get_qos());

    DiscoveredWriterData {
      last_updated: Instant::now(),
      writer_proxy,
      publication_topic_data,
    }
  }

  pub fn update(&mut self, rtps_writer_proxy: &RtpsWriterProxy) {
    self.writer_proxy.remote_writer_guid = Some(rtps_writer_proxy.remote_writer_guid.clone());
    self.writer_proxy.unicast_locator_list = rtps_writer_proxy.unicast_locator_list.clone();
    self.writer_proxy.multicast_locator_list = rtps_writer_proxy.multicast_locator_list.clone();
  }
}

impl<'de> Deserialize<'de> for DiscoveredWriterData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    Ok(res.generate_discovered_writer_data())
  }
}

impl Serialize for DiscoveredWriterData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_discovered_writer_data(&self);
    builtin_data_serializer.serialize::<S>(serializer, true)
  }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TopicBuiltinTopicData {
  pub key: Option<GUID>,
  pub name: Option<String>,
  pub type_name: Option<String>,
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

impl<'de> Deserialize<'de> for TopicBuiltinTopicData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    Ok(res.generate_topic_data())
  }
}

impl Serialize for TopicBuiltinTopicData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_topic_data(&self);
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DiscoveredTopicData {
  pub updated_time: u64,
  pub topic_data: TopicBuiltinTopicData,
}

impl DiscoveredTopicData {
  pub fn new(topic_data: TopicBuiltinTopicData) -> DiscoveredTopicData {
    DiscoveredTopicData {
      updated_time: time::precise_time_ns(),
      topic_data,
    }
  }

  pub fn get_topic_name(&self) -> String {
    match &self.topic_data.name {
      Some(n) => n.clone(),
      None => String::from(""),
    }
  }

  pub fn get_type_name(&self) -> String {
    match &self.topic_data.type_name {
      Some(t) => t.clone(),
      None => String::from(""),
    }
  }
}

impl<'de> Deserialize<'de> for DiscoveredTopicData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_any(custom_ds)?;
    let topic_data = res.generate_topic_data();

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

impl Keyed for DiscoveredTopicData {
  type K = GUID;

  fn get_key(&self) -> Self::K {
    // topic should always have a name, if this crashes the problem is in the overall logic (or message parsing)
    match self.topic_data.key {
      Some(k) => k,
      None => GUID::GUID_UNKNOWN,
    }
  }
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct ParticipantMessageDataKind {
  value: [u8; 4],
}

impl ParticipantMessageDataKind {
  pub const PARTICIPANT_MESSAGE_DATA_KIND_UNKNOWN: ParticipantMessageDataKind =
    ParticipantMessageDataKind {
      value: [0x00, 0x00, 0x00, 0x00],
    };
  pub const PARTICIPANT_MESSAGE_DATA_KIND_AUTOMATIC_LIVELINESS_UPDATE: ParticipantMessageDataKind =
    ParticipantMessageDataKind {
      value: [0x00, 0x00, 0x00, 0x01],
    };
  pub const PARTICIPANT_MESSAGE_DATA_KIND_MANUAL_LIVELINESS_UPDATE: ParticipantMessageDataKind =
    ParticipantMessageDataKind {
      value: [0x00, 0x00, 0x00, 0x02],
    };
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ParticipantMessageData {
  pub guid: GuidPrefix,
  pub kind: ParticipantMessageDataKind,
  // normally this should be empty
  pub length: u32,
  pub data: Vec<u8>,
}

impl Keyed for ParticipantMessageData {
  type K = (GuidPrefix, ParticipantMessageDataKind);

  fn get_key(&self) -> Self::K {
    (self.guid, self.kind)
  }
}

impl Key for (GuidPrefix, ParticipantMessageDataKind) {}

#[cfg(test)]
mod tests {
  use std::{fs::File, io::Read};

  use super::*;

  //use crate::serialization::cdrSerializer::to_little_endian_binary;
  use crate::serialization::{
    Message,
    cdrDeserializer::CDR_deserializer_adapter,
    cdrSerializer::{to_bytes},
  };
  use byteorder::LittleEndian;
  use log::info;
  use crate::serialization::pl_cdr_deserializer::PlCdrDeserializerAdapter;

  use crate::{
    test::test_data::{
      subscription_builtin_topic_data, reader_proxy_data, content_filter_data, writer_proxy_data,
      publication_builtin_topic_data, topic_data,
    },
  };
  use crate::{
    messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier,
    dds::traits::serde_adapters::DeserializerAdapter,
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
    reader_proxy.remote_reader_guid = sub_topic_data.key.clone();
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

    let raw_data = [
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
    ];

    let msg = Message::read_from_buffer(&raw_data).unwrap();
    info!("{:?}", msg);
  }

  #[test]
  fn td_discovered_writer_data_ser_deser() {
    let mut writer_proxy = writer_proxy_data().unwrap();
    let pub_topic_data = publication_builtin_topic_data().unwrap();
    writer_proxy.remote_writer_guid = pub_topic_data.key.clone();

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

  #[test]
  fn td_participant_message_data_ser_deser() {
    let mut pmd_file = File::open("participant_message_data.bin").unwrap();
    let mut buffer: [u8; 1024] = [0; 1024];
    let len = pmd_file.read(&mut buffer).unwrap();

    println!("Buffer: size: {}\n{:?}", len, buffer[..len].to_vec());
    let rpi = CDR_deserializer_adapter::<ParticipantMessageData>::from_bytes(
      &buffer,
      RepresentationIdentifier::CDR_LE,
    )
    .unwrap();
    println!("ParticipantMessageData: \n{:?}", rpi);
    let data2 = to_bytes::<ParticipantMessageData, LittleEndian>(&rpi).unwrap();
    println!("Data2: \n{:?}", data2);
  }
}
