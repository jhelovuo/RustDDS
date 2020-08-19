use std::{
  sync::{Arc, RwLock},
  time::{Duration},
};
use std::ops::Deref;

use mio_extras::channel as mio_channel;
use serde::{Serialize, Serializer};

use crate::structure::time::Timestamp;
use crate::structure::entity::{Entity};
use crate::structure::{dds_cache::DDSCache};

use crate::dds::pubsub::Publisher;
use crate::dds::topic::Topic;
use crate::dds::values::result::{
  Result, LivelinessLostStatus, OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus,
  PublicationMatchedStatus,
};
use crate::dds::traits::dds_entity::DDSEntity;
use crate::dds::traits::key::*;

use crate::dds::qos::{HasQoSPolicy, QosPolicies};
use crate::dds::ddsdata::DDSData;

use crate::{discovery::data_types::topic_data::SubscriptionBuiltinTopicData, dds::datawriter as datawriter_with_key};

// This structure shoud be private to no_key DataWriter
struct NoKeyWrapper_Write<D> {
  pub d: D,
}

// implement Deref so that &NoKeyWrapper_Write<D> is coercible to &D
impl<D> Deref for NoKeyWrapper_Write<D> {
  type Target = D;
  fn deref(&self) -> &Self::Target {
    &self.d
  }
}

impl<D> Keyed for NoKeyWrapper_Write<D> {
  type K = ();
  fn get_key(&self) -> () {
    ()
  }
}

impl<D> Serialize for NoKeyWrapper_Write<D>
where
  D: Serialize,
{
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    self.d.serialize(serializer)
  }
}

impl<D> NoKeyWrapper_Write<D> {}

pub struct DataWriter<'a, D> {
  keyed_datawriter: datawriter_with_key::DataWriter<'a, NoKeyWrapper_Write<D>>,
}

impl<'a, D> DataWriter<'a, D>
where
  D: Serialize,
{
  pub fn new(
    publisher: &'a Publisher,
    topic: &'a Topic,
    cc_upload: mio_channel::Sender<DDSData>,
    dds_cache: Arc<RwLock<DDSCache>>,
  ) -> DataWriter<'a, D> {
    DataWriter {
      keyed_datawriter: datawriter_with_key::DataWriter::<'a, NoKeyWrapper_Write<D>>::new(
        publisher, topic, cc_upload, dds_cache,
      ),
    }
  }

  // write (with optional timestamp)
  pub fn write(&mut self, data: D, source_timestamp: Option<Timestamp>) -> Result<()> {
    self
      .keyed_datawriter
      .write(NoKeyWrapper_Write::<D> { d: data }, source_timestamp)
  }

  // dispose
  // The data item is given only for identification, i.e. extracting the key
  // NO_KEY data cannot be disposed. So this method should not even exist.
  // pub fn dispose(
  //   &mut self,
  //   key: <D as Keyed>::K,
  //   source_timestamp: Option<Timestamp>,
  // ) -> Result<()> {

  // }

  pub fn wait_for_acknowledgments(&self, max_wait: Duration) -> Result<()> {
    self.keyed_datawriter.wait_for_acknowledgments(max_wait)
  }

  // status queries
  pub fn get_liveliness_lost_status(&self) -> Result<LivelinessLostStatus> {
    self.keyed_datawriter.get_liveliness_lost_status()
  }
  pub fn get_offered_deadline_missed_status(&self) -> Result<OfferedDeadlineMissedStatus> {
    self.keyed_datawriter.get_offered_deadline_missed_status()
  }
  pub fn get_offered_incompatible_qos_status(&self) -> Result<OfferedIncompatibleQosStatus> {
    self.keyed_datawriter.get_offered_incompatible_qos_status()
  }
  pub fn get_publication_matched_status(&self) -> Result<PublicationMatchedStatus> {
    self.keyed_datawriter.get_publication_matched_status()
  }

  // who are we connected to?
  pub fn get_topic(&self) -> &Topic {
    &self.keyed_datawriter.get_topic()
  }
  pub fn get_publisher(&self) -> &Publisher {
    &self.keyed_datawriter.get_publisher()
  }

  pub fn assert_liveliness(&self) -> Result<()> {
    self.keyed_datawriter.assert_liveliness()
  }

  pub fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData> {
    self.keyed_datawriter.get_matched_subscriptions()
  }
}

impl<D> Entity for DataWriter<'_, D> {
  fn as_entity(&self) -> &crate::structure::entity::EntityAttributes {
    self.keyed_datawriter.as_entity()
  }
}

impl<D> HasQoSPolicy for DataWriter<'_, D> {
  fn set_qos(&mut self, policy: &QosPolicies) -> Result<()> {
    self.keyed_datawriter.set_qos(policy)
  }

  fn get_qos(&self) -> &QosPolicies {
    self.keyed_datawriter.get_qos()
  }
}

impl<D> DDSEntity for DataWriter<'_, D> {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dds::participant::DomainParticipant;
  use crate::dds::typedesc::TypeDesc;
  use crate::test::random_data::*;
  use std::thread;
  use crate::dds::traits::key::Keyed;

  #[test]
  fn dw_write_test() {
    let domain_participant = DomainParticipant::new(9, 0);
    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos)
      .expect("Failed to create topic");

    let mut data_writer = publisher
      .create_datawriter(None, &topic, &qos)
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
    let domain_participant = DomainParticipant::new(7, 0);
    let qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos)
      .expect("Failed to create topic");

    let mut data_writer = publisher
      .create_datawriter(None, &topic, &qos)
      .expect("Failed to create datawriter");

    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    let key = &data.get_key().get_hash();
    println!();
    println!("key: {:?}", key);
    println!();
    println!();
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());

    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    data_writer
      .write(data.clone(), None)
      .expect("Unable to write data");

    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    data_writer
      .dispose(data.get_key(), None)
      .expect("Unable to dispose data");

    // TODO: verify that dispose is sent correctly
  }

  #[test]
  fn dw_wait_for_ack_test() {
    let domain_participant = DomainParticipant::new(8, 0);
    let qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos)
      .expect("Failed to create topic");

    let mut data_writer = publisher
      .create_datawriter(None, &topic, &qos)
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
