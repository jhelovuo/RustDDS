use std::time::Duration;
use serde::Serialize;
use mio_extras::channel as mio_channel;

use crate::structure::time::Timestamp;
use crate::structure::entity::{Entity, EntityAttributes};
use crate::structure::guid::{GUID, EntityId};

use crate::dds::pubsub::Publisher;
use crate::dds::topic::Topic;
use crate::dds::participant::SubscriptionBuiltinTopicData;
use crate::dds::traits::key::Keyed;
use crate::dds::values::result::{
  Result, Error, LivelinessLostStatus, OfferedDeadlineMissedStatus, OfferedIncompatibelQosStatus,
  PublicationMatchedStatus,
};
use crate::dds::traits::dds_entity::DDSEntity;
use crate::dds::traits::datasample_trait::DataSampleTrait;
use crate::dds::qos::{HasQoSPolicy, QosPolicies};
use crate::dds::datasample_cache::DataSampleCache;
use crate::dds::datasample::DataSample;
use crate::dds::ddsdata::DDSData;

pub struct DataWriter<'p> {
  my_publisher: &'p Publisher<'p>,
  my_topic: &'p Topic<'p>,
  qos_policy: &'p QosPolicies,
  entity_attributes: EntityAttributes,
  cc_upload: mio_channel::Sender<DDSData>,
  datasample_cache: DataSampleCache,
}

impl<'p> DataWriter<'p> {
  pub fn new(
    publisher: &'p Publisher,
    topic: &'p Topic,
    cc_upload: mio_channel::Sender<DDSData>,
  ) -> DataWriter<'p> {
    let entity_attributes = EntityAttributes::new(GUID::new_with_prefix_and_id(
      publisher.get_participant().get_guid_prefix(),
      EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
    ));

    DataWriter {
      my_publisher: publisher,
      my_topic: topic,
      qos_policy: topic.get_qos(),
      entity_attributes,
      cc_upload,
      datasample_cache: DataSampleCache::new(topic.get_qos().clone()),
    }
  }

  // Instance registration operations:
  // * register_instance (_with_timestamp)
  // * unregister_instance (_with_timestamp)
  // * get_key_value  (InstanceHandle --> Key)
  // * lookup_instance (Key --> InstanceHandle)
  // Do not implement these until there is a clear use case for InstanceHandle type.

  // write (with optional timestamp)
  // This operation could take also in InstanceHandle, if we would use them.
  // The _with_timestamp version is covered by the optional timestamp.
  pub fn write<D: 'static>(&mut self, data: D, source_timestamp: Option<Timestamp>) -> Result<()>
  where
    D: DataSampleTrait + Clone,
  {
    let ddsdata = DDSData::from(&data, source_timestamp);

    let data_sample = match source_timestamp {
      Some(t) => DataSample::new(t, data),
      None => DataSample::new(Timestamp::from(time::get_time()), data),
    };

    let _key = self.datasample_cache.add_datasample::<D>(data_sample)?;

    match self.cc_upload.send(ddsdata) {
      Ok(_) => Ok(()),
      _ => Err(Error::OutOfResources),
    }
  }

  // dispose
  // The data item is given only for identification, i.e. extracting the key
  pub fn dispose<D>(&self, data: &D, _source_timestamp: Option<Timestamp>)
  where
    D: Send + Sync + Serialize + Keyed,
  {
    let _key = data.get_key().get_hash();
    // let d = self.datasample_cache.
  }

  pub fn wait_for_acknowledgments(&self, _max_wait: Duration) -> Result<()> {
    unimplemented!();
  }

  // status queries
  pub fn get_liveliness_lost_status(&self) -> Result<LivelinessLostStatus> {
    unimplemented!()
  }
  pub fn get_offered_deadline_missed_status(&self) -> Result<OfferedDeadlineMissedStatus> {
    unimplemented!()
  }
  pub fn get_offered_incompatibel_qos_status(&self) -> Result<OfferedIncompatibelQosStatus> {
    unimplemented!()
  }
  pub fn get_publication_matched_status(&self) -> Result<PublicationMatchedStatus> {
    unimplemented!()
  }

  // who are we connected to?
  pub fn get_topic(&self) -> &Topic {
    self.my_topic
  }
  pub fn get_publisher(&self) -> &Publisher {
    self.my_publisher
  }

  pub fn assert_liveliness(&self) -> Result<()> {
    unimplemented!()
  }

  // This shoudl really return InstanceHandles pointing to a BuiltInTopic reader
  //  but let's see if we can do without those handles.
  pub fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData> {
    unimplemented!()
  }
  // This one function provides both get_matched_subscrptions and get_matched_subscription_data
  // TODO: Maybe we could return references to the subscription data to avoid copying?
  // But then what if the result set changes while the application processes it?
}

impl<'a> Entity for DataWriter<'a> {
  fn as_entity(&self) -> &crate::structure::entity::EntityAttributes {
    &self.entity_attributes
  }
}

impl<'p> HasQoSPolicy<'p> for DataWriter<'p> {
  fn set_qos(mut self, policy: &'p QosPolicies) -> Result<()> {
    // TODO: check liveliness of qos_policy
    self.qos_policy = policy;
    Ok(())
  }

  fn get_qos(&self) -> &'p QosPolicies {
    self.qos_policy
  }
}

impl<'a> DDSEntity<'a> for DataWriter<'a> {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dds::participant::DomainParticipant;
  use crate::dds::typedesc::TypeDesc;
  use crate::dds::traits::key::Key;
  use std::hash::{Hash, Hasher};
  use std::collections::hash_map::DefaultHasher;
  struct RandomKey {
    val: i64,
  }

  impl RandomKey {
    pub fn new(val: i64) -> RandomKey {
      RandomKey { val }
    }
  }

  impl Key for RandomKey {
    fn get_hash(&self) -> u64 {
      let mut hasher = DefaultHasher::new();
      self.val.hash(&mut hasher);
      hasher.finish()
    }

    fn box_clone(&self) -> Box<dyn Key> {
      let n = RandomKey::new(self.val);
      Box::new(n)
    }
  }

  #[derive(Serialize, Clone)]
  struct RandomData {
    a: i64,
    b: String,
  }

  impl Keyed for RandomData {
    fn get_key(&self) -> Box<dyn Key> {
      let key = RandomKey::new(self.a);
      Box::new(key)
    }
  }

  impl DataSampleTrait for RandomData {
    fn box_clone(&self) -> Box<dyn DataSampleTrait> {
      Box::new(RandomData {
        a: self.a.clone(),
        b: self.b.clone(),
      })
    }
  }

  #[test]
  fn dw_basic_test() {
    let domain_participant = DomainParticipant::new();
    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(qos.clone())
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), qos.clone())
      .expect("Failed to create topic");

    let mut data_writer = publisher
      .create_datawriter(&topic, qos.clone())
      .expect("Failed to create datawriter");

    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    data_writer.write(data, None).expect("Unable to write data");

    // TODO: verify that data is sent/writtent correctly
    // TODO: write also with timestamp
  }
}
