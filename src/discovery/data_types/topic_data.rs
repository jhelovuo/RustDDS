use serde::{Serialize, Deserialize};

use crate::{
  serialization::{
    builtin_data_serializer::BuiltinDataSerializer,
    builtin_data_deserializer::BuiltinDataDeserializer,
  },
  structure::{locator::LocatorList, guid::GUID},
  dds::{
    qos::policy::{
      Deadline, Durability, LatencyBudget, Reliability, Ownership, DestinationOrder, Liveliness,
      TimeBasedFilter, Presentation, Lifespan, History, ResourceLimits,
    },
    traits::key::Keyed,
  },
  discovery::content_filter_property::ContentFilterProperty,
};

// Topic data contains all topic related (including reader and writer data structures for serialization and deserialization)
#[derive(Debug, Clone, PartialEq)]
pub struct ReaderProxy {
  pub remote_reader_guid: Option<GUID>,
  pub expects_inline_qos: Option<bool>,
  pub unicast_locator_list: LocatorList,
  pub multicast_locator_list: LocatorList,
}

impl<'de> Deserialize<'de> for ReaderProxy {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_byte_buf(custom_ds).unwrap();
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

#[derive(Debug, Clone, PartialEq)]
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

impl<'de> Deserialize<'de> for SubscriptionBuiltinTopicData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_byte_buf(custom_ds).unwrap();
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

#[derive(Debug, PartialEq)]
pub struct DiscoveredReaderData {
  pub reader_proxy: ReaderProxy,
  pub subscription_topic_data: SubscriptionBuiltinTopicData,
  pub content_filter: Option<ContentFilterProperty>,
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
    let res = deserializer.deserialize_byte_buf(custom_ds).unwrap();
    Ok(res.generate_discovered_reader_data())
  }
}

impl Serialize for DiscoveredReaderData {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let builtin_data_serializer = BuiltinDataSerializer::from_discovered_reader_data(&self);
    builtin_data_serializer.serialize::<S>(serializer, true)
  }
}

#[derive(Debug, PartialEq)]
pub struct WriterProxy {
  pub remote_writer_guid: Option<GUID>,
  pub unicast_locator_list: LocatorList,
  pub multicast_locator_list: LocatorList,
  pub data_max_size_serialized: Option<u32>,
}

impl<'de> Deserialize<'de> for WriterProxy {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_byte_buf(custom_ds).unwrap();
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

#[derive(Debug, PartialEq)]
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

impl<'de> Deserialize<'de> for PublicationBuiltinTopicData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_byte_buf(custom_ds).unwrap();
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

#[derive(Debug, PartialEq)]
pub struct DiscoveredWriterData {
  pub writer_proxy: WriterProxy,
  pub publication_topic_data: PublicationBuiltinTopicData,
}

impl<'de> Deserialize<'de> for DiscoveredWriterData {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let custom_ds = BuiltinDataDeserializer::new();
    let res = deserializer.deserialize_byte_buf(custom_ds).unwrap();
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

#[derive(Debug, PartialEq)]
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
    let res = deserializer.deserialize_byte_buf(custom_ds).unwrap();
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

#[derive(Debug, PartialEq, Deserialize)]
pub struct DiscoveredTopicData {
  topic_data: TopicBuiltinTopicData,
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

#[cfg(test)]
mod tests {
  use super::*;

  //use crate::serialization::cdrSerializer::to_little_endian_binary;
  use crate::serialization::cdrSerializer::{to_bytes};
  use byteorder::LittleEndian;
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;


  use crate::{
    test::test_data::{
      subscription_builtin_topic_data, reader_proxy_data, content_filter_data, writer_proxy_data,
      publication_builtin_topic_data, topic_data,
    },
  };

  #[test]
  fn td_reader_proxy_ser_deser() {
    let reader_proxy = reader_proxy_data().unwrap();

    let sdata = to_bytes::<ReaderProxy,LittleEndian>(&reader_proxy).unwrap();
    let reader_proxy2: ReaderProxy = deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(reader_proxy, reader_proxy2);
    let sdata2 = to_bytes::<ReaderProxy,LittleEndian>(&reader_proxy2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_writer_proxy_ser_deser() {
    let writer_proxy = writer_proxy_data().unwrap();

    let sdata = to_bytes::<WriterProxy,LittleEndian>(&writer_proxy).unwrap();
    let writer_proxy2: WriterProxy = deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(writer_proxy, writer_proxy2);
    let sdata2 = to_bytes::<WriterProxy, LittleEndian>(&writer_proxy2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_subscription_builtin_topic_data_ser_deser() {
    let sub_topic_data = subscription_builtin_topic_data().unwrap();

    let sdata = to_bytes::<SubscriptionBuiltinTopicData,LittleEndian>(&sub_topic_data).unwrap();
    let sub_topic_data2: SubscriptionBuiltinTopicData =
      deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(sub_topic_data, sub_topic_data2);
    let sdata2 = to_bytes::<SubscriptionBuiltinTopicData,LittleEndian>(&sub_topic_data2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_publication_builtin_topic_data_ser_deser() {
    let pub_topic_data = publication_builtin_topic_data().unwrap();

    let sdata = to_bytes::<PublicationBuiltinTopicData,LittleEndian>(&pub_topic_data).unwrap();
    let pub_topic_data2: PublicationBuiltinTopicData =
      deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(pub_topic_data, pub_topic_data2);
    let sdata2 = to_bytes::<PublicationBuiltinTopicData,LittleEndian>(&pub_topic_data2).unwrap();
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

    let sdata = to_bytes::<DiscoveredReaderData,LittleEndian>(&drd).unwrap();
    let drd2: DiscoveredReaderData = deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(drd, drd2);
    let sdata2 = to_bytes::<DiscoveredReaderData,LittleEndian>(&drd2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_discovered_writer_data_ser_deser() {
    let mut writer_proxy = writer_proxy_data().unwrap();
    let pub_topic_data = publication_builtin_topic_data().unwrap();
    writer_proxy.remote_writer_guid = pub_topic_data.key.clone();

    let dwd = DiscoveredWriterData {
      writer_proxy,
      publication_topic_data: pub_topic_data,
    };

    let sdata = to_bytes::<DiscoveredWriterData,LittleEndian>(&dwd).unwrap();
    let dwd2: DiscoveredWriterData = deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(dwd, dwd2);
    let sdata2 = to_bytes::<DiscoveredWriterData,LittleEndian>(&dwd2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_topic_data_ser_deser() {
    let topic_data = topic_data().unwrap();

    let sdata = to_bytes::<TopicBuiltinTopicData,LittleEndian>(&topic_data).unwrap();
    let topic_data2: TopicBuiltinTopicData = deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(topic_data, topic_data2);
    let sdata2 = to_bytes::<TopicBuiltinTopicData,LittleEndian>(&topic_data2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_discovered_topic_data_ser_deser() {
    let topic_data = topic_data().unwrap();

    let dtd = DiscoveredTopicData { topic_data };

    let sdata = to_bytes::<DiscoveredTopicData,LittleEndian>(&dtd).unwrap();
    let dtd2: DiscoveredTopicData = deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(dtd, dtd2);
    let sdata2 = to_bytes::<DiscoveredTopicData,LittleEndian>(&dtd2).unwrap();
    assert_eq!(sdata, sdata2);
  }
}
