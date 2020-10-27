use std::{
  //sync::{Arc, RwLock},
  time::{Duration},
};

//use mio_extras::channel as mio_channel;
use mio_extras::channel::Receiver;
use serde::Serialize;

use crate::{
  dds::interfaces::IDataWriter, dds::values::result::StatusChange, structure::time::Timestamp,
};
use crate::structure::entity::{Entity};
//use crate::structure::{dds_cache::DDSCache, guid::{GUID} };

use crate::dds::pubsub::Publisher;
use crate::dds::topic::Topic;
use crate::dds::values::result::{
  Result, LivelinessLostStatus, OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus,
  PublicationMatchedStatus,
};
use crate::dds::traits::dds_entity::DDSEntity;
use crate::dds::traits::serde_adapters::SerializerAdapter;

use crate::dds::qos::{HasQoSPolicy, QosPolicies};
//use crate::dds::ddsdata::DDSData;

use crate::{
  discovery::data_types::topic_data::SubscriptionBuiltinTopicData,
  dds::datawriter as datawriter_with_key,
};

use super::wrappers::{NoKeyWrapper, SAWrapper};

pub struct DataWriter<'a, D: Serialize, SA: SerializerAdapter<D>> {
  keyed_datawriter: datawriter_with_key::DataWriter<'a, NoKeyWrapper<D>, SAWrapper<SA>>,
}

impl<'a, D, SA> DataWriter<'a, D, SA>
where
  D: Serialize,
  SA: SerializerAdapter<D>,
{
  /*
  pub fn new(
    publisher: &'a Publisher,
    topic: &'a Topic,
    guid: Option<GUID>,
    cc_upload: mio_channel::Sender<DDSData>,
    dds_cache: Arc<RwLock<DDSCache>>,
  ) -> Result<DataWriter<'a, D, SA>> {
    Ok( DataWriter {
      keyed_datawriter: datawriter_with_key::DataWriter::<'a, NoKeyWrapper<D>, SAWrapper<SA>>::new(
        publisher, topic, guid, cc_upload, dds_cache)?,
    })
  }
  */
  pub(crate) fn from_keyed(
    keyed: datawriter_with_key::DataWriter<'a, NoKeyWrapper<D>, SAWrapper<SA>>,
  ) -> DataWriter<'a, D, SA> {
    DataWriter {
      keyed_datawriter: keyed,
    }
  }
}

impl<D: Serialize, SA: SerializerAdapter<D>> IDataWriter<D, SA> for DataWriter<'_, D, SA> {
  // write (with optional timestamp)
  fn write(&self, data: D, source_timestamp: Option<Timestamp>) -> Result<()> {
    self
      .keyed_datawriter
      .write(NoKeyWrapper::<D> { d: data }, source_timestamp)
  }

  fn wait_for_acknowledgments(&self, max_wait: Duration) -> Result<()> {
    self.keyed_datawriter.wait_for_acknowledgments(max_wait)
  }

  // status queries
  fn get_liveliness_lost_status(&self) -> Result<LivelinessLostStatus> {
    self.keyed_datawriter.get_liveliness_lost_status()
  }

  fn get_offered_deadline_missed_status(&self) -> Result<OfferedDeadlineMissedStatus> {
    self.keyed_datawriter.get_offered_deadline_missed_status()
  }

  fn get_offered_incompatible_qos_status(&self) -> Result<OfferedIncompatibleQosStatus> {
    self.keyed_datawriter.get_offered_incompatible_qos_status()
  }

  fn get_publication_matched_status(&self) -> Result<PublicationMatchedStatus> {
    self.keyed_datawriter.get_publication_matched_status()
  }

  // who are we connected to?
  fn get_topic(&self) -> &Topic {
    &self.keyed_datawriter.get_topic()
  }

  fn get_publisher(&self) -> &Publisher {
    &self.keyed_datawriter.get_publisher()
  }

  fn assert_liveliness(&self) -> Result<()> {
    self.keyed_datawriter.assert_liveliness()
  }

  fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData> {
    self.keyed_datawriter.get_matched_subscriptions()
  }

  fn get_status_listener(&self) -> &Receiver<StatusChange> {
    self.keyed_datawriter.get_status_listener()
  }
}

impl<D: Serialize, SA: SerializerAdapter<D>> Entity for DataWriter<'_, D, SA> {
  fn as_entity(&self) -> &crate::structure::entity::EntityAttributes {
    self.keyed_datawriter.as_entity()
  }
}

impl<D: Serialize, SA: SerializerAdapter<D>> HasQoSPolicy for DataWriter<'_, D, SA> {
  fn set_qos(&mut self, policy: &QosPolicies) -> Result<()> {
    self.keyed_datawriter.set_qos(policy)
  }

  fn get_qos(&self) -> &QosPolicies {
    self.keyed_datawriter.get_qos()
  }
}

impl<D: Serialize, SA: SerializerAdapter<D>> DDSEntity for DataWriter<'_, D, SA> {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dds::{participant::DomainParticipant, traits::key::Key};
  use crate::dds::typedesc::TypeDesc;
  use crate::test::random_data::*;
  use std::thread;
  use crate::dds::traits::key::Keyed;
  use crate::serialization::cdrSerializer::*;
  use byteorder::LittleEndian;
  use log::info;

  #[test]
  fn dw_write_test() {
    let domain_participant = DomainParticipant::new(0);
    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos)
      .expect("Failed to create topic");

    let data_writer: DataWriter<'_, RandomData, CDR_serializer_adapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter_no_key(None, &topic, qos)
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
    let domain_participant = DomainParticipant::new(0);
    let qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos)
      .expect("Failed to create topic");

    let data_writer: DataWriter<'_, RandomData, CDR_serializer_adapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter_no_key(None, &topic, qos)
        .expect("Failed to create datawriter");

    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    let key = &data.get_key().into_hash_key();
    info!("key: {:?}", key);
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());

    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    data_writer
      .write(data.clone(), None)
      .expect("Unable to write data");

    /* Keyless topics cannot dispose.
    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    data_writer
      .dispose(data.get_key(), None)
      .expect("Unable to dispose data");

    // TODO: verify that dispose is sent correctly
    */
  }

  #[test]
  fn dw_wait_for_ack_test() {
    let domain_participant = DomainParticipant::new(0);
    let qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos)
      .expect("Failed to create topic");

    let data_writer: DataWriter<'_, RandomData, CDR_serializer_adapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter_no_key(None, &topic, qos)
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
