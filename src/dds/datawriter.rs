use std::time::Duration;
use mio_extras::channel as mio_channel;

use crate::structure::time::Timestamp;
use crate::structure::entity::{Entity, EntityAttributes};
use crate::structure::guid::{GUID, EntityId};

use crate::dds::pubsub::Publisher;
use crate::dds::topic::Topic;
use crate::dds::participant::SubscriptionBuiltinTopicData;
use crate::dds::values::result::{
  Result, Error, LivelinessLostStatus, OfferedDeadlineMissedStatus, OfferedIncompatibelQosStatus,
  PublicationMatchedStatus,
};
use crate::dds::traits::dds_entity::DDSEntity;
use crate::dds::traits::datasample_trait::DataSampleTrait;
use crate::dds::qos::{
  HasQoSPolicy, QosPolicies,
  policy::{Reliability},
};
use crate::dds::datasample_cache::DataSampleCache;
use crate::dds::datasample::DataSample;
use crate::dds::ddsdata::DDSData;

pub struct DataWriter<'p,D> {
  my_publisher: &'p Publisher<'p>,
  my_topic: &'p Topic<'p>,
  qos_policy: &'p QosPolicies,
  entity_attributes: EntityAttributes,
  cc_upload: mio_channel::Sender<DDSData>,
  datasample_cache: DataSampleCache<D>,
}

impl<'p,D> DataWriter<'p,D> 
  where
    D: DataSampleTrait + Clone,
{
  pub fn new(
    publisher: &'p Publisher,
    topic: &'p Topic,
    cc_upload: mio_channel::Sender<DDSData>,
  ) -> DataWriter<'p,D> {
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
  pub fn write(&mut self, data: D, source_timestamp: Option<Timestamp>) -> Result<()>
  {
    let instance_handle = self.datasample_cache.generate_free_instance_handle();
    let ddsdata = DDSData::from(instance_handle.clone(), &data, source_timestamp);

    let data_sample = match source_timestamp {
      Some(t) => DataSample::new(t, instance_handle, data),
      None => DataSample::new(Timestamp::from(time::get_time()), instance_handle, data),
    };

    let _key = self.datasample_cache.add_datasample(data_sample)?;

    match self.cc_upload.send(ddsdata) {
      Ok(_) => Ok(()),
      _ => Err(Error::OutOfResources),
    }
  }

  // dispose
  // The data item is given only for identification, i.e. extracting the key
  pub fn dispose(&mut self, data: &D, source_timestamp: Option<Timestamp>) -> Result<()>
  {
    let instance_handle = self.datasample_cache.generate_free_instance_handle();
    let ddsdata = DDSData::from_dispose(instance_handle.clone(), &data, source_timestamp);

    let data_sample = match source_timestamp {
      Some(t) => DataSample::new_disposed(t, instance_handle, data),
      None => DataSample::new_disposed(Timestamp::from(time::get_time()), instance_handle, data),
    };

    let _key = self.datasample_cache.add_datasample(data_sample)?;

    match self.cc_upload.send(ddsdata) {
      Ok(_) => Ok(()),
      Err(huh) => {
        println!("Error: {:?}", huh);
        Err(Error::OutOfResources)
      }
    }
  }

  pub fn wait_for_acknowledgments(&self, _max_wait: Duration) -> Result<()> {
    match &self.qos_policy.reliability {
      Some(rel) => match rel {
        Reliability::BestEffort => return Ok(()),
        Reliability::Reliable {
          max_blocking_time: _,
        } =>
        // TODO: implement actual waiting for acks
        {
          ()
        }
      },
      None => return Ok(()),
    };

    // TODO: wait for actual acknowledgements to writers writes
    return Err(Error::Unsupported);
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
    // TODO: probably needs extra liveliness mio_channel to writer
    unimplemented!()
  }

  // This should really return InstanceHandles pointing to a BuiltInTopic reader
  //  but let's see if we can do without those handles.
  pub fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData> {
    unimplemented!()
  }
  // This one function provides both get_matched_subscrptions and get_matched_subscription_data
  // TODO: Maybe we could return references to the subscription data to avoid copying?
  // But then what if the result set changes while the application processes it?
}

impl<'a,D> Entity for DataWriter<'a,D> {
  fn as_entity(&self) -> &crate::structure::entity::EntityAttributes {
    &self.entity_attributes
  }
}

impl<'p,D> HasQoSPolicy<'p> for DataWriter<'p,D> {
  fn set_qos(mut self, policy: &'p QosPolicies) -> Result<()> {
    // TODO: check liveliness of qos_policy
    self.qos_policy = policy;
    Ok(())
  }

  fn get_qos(&self) -> &'p QosPolicies {
    self.qos_policy
  }
}

impl<'a,D> DDSEntity<'a> for DataWriter<'a,D> {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dds::participant::DomainParticipant;
  use crate::dds::typedesc::TypeDesc;
  use crate::test::random_data::*;

  #[test]
  fn dw_write_test() {
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

    let mut data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    data_writer
      .write(data.clone(), None)
      .expect("Unable to write data");

    data.a = 5;
    let timestamp: Timestamp = Timestamp::from(time::get_time());
    data_writer
      .write(data, Some(timestamp))
      .expect("Unable to write data with timestamp");

    // TODO: verify that data is sent/writtent correctly
    // TODO: write also with timestamp
  }

  #[test]
  fn dw_dispose_test() {
    let domain_participant = DomainParticipant::new();
    let qos = QosPolicies::qos_none();
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

    data_writer
      .write(data.clone(), None)
      .expect("Unable to write data");

    data_writer
      .dispose(&data, None)
      .expect("Unable to dispose data");

    // TODO: verify that dispose is sent correctly
  }

  #[test]
  fn dw_wait_for_ack_test() {
    let domain_participant = DomainParticipant::new();
    let qos = QosPolicies::qos_none();
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

    let res = data_writer
      .wait_for_acknowledgments(Duration::from_secs(5))
      .unwrap();
    assert_eq!(res, ());
  }
}
