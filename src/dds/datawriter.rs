use std::time::Duration;
use serde::Serialize;
use mio_extras::channel as mio_channel;
use rand::Rng;

use crate::structure::time::Timestamp;
use crate::structure::entity::{Entity, EntityAttributes};
use crate::structure::guid::{GUID, EntityId};

use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;

use crate::serialization::cdrSerializer::to_little_endian_binary;

use crate::dds::pubsub::Publisher;
use crate::dds::topic::Topic;
use crate::dds::participant::SubscriptionBuiltinTopicData;
use crate::dds::traits::key::{Keyed, DefaultKey};
use crate::dds::traits::named::Named;
use crate::dds::values::result::{
  Result, LivelinessLostStatus, OfferedDeadlineMissedStatus, OfferedIncompatibelQosStatus,
  PublicationMatchedStatus,
};
use crate::dds::traits::dds_entity::DDSEntity;
use crate::dds::qos::{HasQoSPolicy, QosPolicies};
use crate::dds::datasample::DataSample;
use crate::dds::datasample_cache::DataSampleCache;
use crate::dds::ddsdata::DDSData;

pub struct DataWriter<'p> {
  my_publisher: &'p Publisher<'p>,
  my_topic: &'p Topic<'p>,
  qos_policy: &'p QosPolicies,
  entity_attributes: EntityAttributes,
  cc_upload: mio_channel::Sender<DataSample<DDSData>>,
  datasample_cache: DataSampleCache,
}

impl<'p> DataWriter<'p> {
  pub fn new(
    publisher: &'p Publisher,
    topic: &'p Topic,
    cc_upload: mio_channel::Sender<DataSample<DDSData>>,
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
      datasample_cache: DataSampleCache::new(),
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
  pub fn write<D>(&mut self, data: D, source_timestamp: Option<Timestamp>) -> Result<()>
  where
    D: Serialize + Keyed + Named,
  {
    // TODO: handle unwrap
    let serialized_payload = to_little_endian_binary(&data).unwrap();
    let serialized_payload = SerializedPayload {
      representation_identifier: D::identifier(),
      representation_options: 0,
      value: serialized_payload,
    };

    // implement systematic way to generate keys
    let mut rng = rand::thread_rng();
    let ddsdata = if serialized_payload.value.len() > 0 {
      Some(DDSData::new(DefaultKey::new(rng.gen()), serialized_payload))
    } else {
      None
    };
    let datasample = match source_timestamp {
      Some(t) => DataSample::new(t, ddsdata),
      None => DataSample::new(Timestamp::from(time::get_time()), ddsdata),
    };

    self
      .cc_upload
      .send(datasample)
      .expect("Unable to send new cache change");
    Ok(())
  }

  // dispose
  // The data item is given only for identification, i.e. extracting the key
  pub fn dispose<D>(&self, _data: &D, _source_timestamp: Option<Timestamp>)
  where
    D: Serialize + Keyed,
  {
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
  use crate::dds::traits::key::DefaultKey;
  use crate::dds::participant::DomainParticipant;
  use crate::dds::typedesc::TypeDesc;

  #[derive(Serialize)]
  struct RandomData {
    key: DefaultKey,
    a: i64,
    b: String,
  }

  impl Keyed for RandomData {
    type K = DefaultKey;
    fn get_key(&self) -> &Self::K {
      &self.key
    }

    fn default() -> Self {
      RandomData {
        key: DefaultKey::default(),
        a: 0,
        b: "".to_string(),
      }
    }
  }

  impl Named for RandomData {
    fn name() -> String {
      "RandomData".to_string()
    }

    fn identifier() -> u16 {
      1
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
      key: DefaultKey::random_key(),
      a: 4,
      b: "Fobar".to_string(),
    };

    data_writer.write(data, None).expect("Unable to write data");

    // TODO: verify that data is sent/writtent correctly
  }
}
