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
      TimeBasedFilter, Presentation, Lifespan,
    },
  },
  discovery::content_filter_property::ContentFilterProperty,
};

// Topic data contains all topic related (including reader and writer data structures for serialization and deserialization)
#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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
    builtin_data_serializer.serialize::<S>(serializer, false)
  }
}

pub struct WriterProxy {
  remote_writer_guid: Option<GUID>,
  unicast_locator_list: LocatorList,
  multicast_locator_list: LocatorList,
  data_max_size_serialized: Option<u64>,
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::serialization::cdrSerializer::to_little_endian_binary;
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;

  use crate::{
    test::test_data::{subscription_builtin_topic_data, reader_proxy_data, content_filter_data},
  };

  #[test]
  fn td_reader_proxy_ser_deser() {
    let reader_proxy = reader_proxy_data().unwrap();

    let sdata = to_little_endian_binary(&reader_proxy).unwrap();
    let reader_proxy2: ReaderProxy = deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(reader_proxy, reader_proxy2);
    let sdata2 = to_little_endian_binary(&reader_proxy2).unwrap();
    assert_eq!(sdata, sdata2);
  }

  #[test]
  fn td_subscription_builtin_topic_data_ser_deser() {
    // TODO: uncomment when enum serialization is complete or QosPolicies have been reworked

    // let sub_topic_data = SubscriptionBuiltinTopicData {
    //   key: Some(GUID::new()),
    //   participant_key: Some(GUID::new()),
    //   topic_name: Some("some topic name".to_string()),
    //   type_name: Some("RandomData".to_string()),
    //   durability: Some(Durability::Transient),
    //   deadline: Some(Deadline {
    //     period: Duration::from(std::time::Duration::from_secs(7)),
    //   }),
    //   latency_budget: Some(LatencyBudget {
    //     duration: Duration::from(std::time::Duration::from_secs(5)),
    //   }),
    //   liveliness: Some(Liveliness {
    //     kind: LivelinessKind::ManulByTopic,
    //     lease_duration: Duration::from(std::time::Duration::from_secs(60)),
    //   }),
    //   reliability: Some(Reliability::BestEffort),
    //   ownership: Some(Ownership::Exclusive { strength: 4 }),
    //   destination_order: Some(DestinationOrder::BySourceTimeStamp),
    //   time_based_filter: Some(TimeBasedFilter {
    //     minimum_separation: Duration::from(std::time::Duration::from_secs(120)),
    //   }),
    //   presentation: Some(Presentation {
    //     access_scope: PresentationAccessScope::Instance,
    //     coherent_access: false,
    //     ordered_access: true,
    //   }),
    //   lifespan: Some(Lifespan {
    //     duration: Duration::from(std::time::Duration::from_secs(3214)),
    //   }),
    // };
    let sub_topic_data = subscription_builtin_topic_data().unwrap();

    let sdata = to_little_endian_binary(&sub_topic_data).unwrap();
    let sub_topic_data2: SubscriptionBuiltinTopicData =
      deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(sub_topic_data, sub_topic_data2);
    let sdata2 = to_little_endian_binary(&sub_topic_data2).unwrap();
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

    let sdata = to_little_endian_binary(&drd).unwrap();
    let drd2: DiscoveredReaderData = deserialize_from_little_endian(&sdata).unwrap();
    assert_eq!(drd, drd2);
    let sdata2 = to_little_endian_binary(&drd2).unwrap();
    assert_eq!(sdata, sdata2);
  }
}
