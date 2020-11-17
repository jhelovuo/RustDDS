use std::{
  time::{Duration},
};

use mio_extras::channel::Receiver;
use serde::Serialize;

use crate::{
  serialization::CDRSerializerAdapter, dds::values::result::StatusChange,
  structure::time::Timestamp,
};
use crate::structure::entity::{Entity};

use crate::dds::pubsub::Publisher;
use crate::dds::topic::Topic;
use crate::dds::values::result::{
  Result, LivelinessLostStatus, OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus,
  PublicationMatchedStatus,
};
use crate::dds::traits::dds_entity::DDSEntity;
use crate::dds::traits::serde_adapters::SerializerAdapter;

use crate::dds::qos::{HasQoSPolicy, QosPolicies};

use crate::{
  discovery::data_types::topic_data::SubscriptionBuiltinTopicData,
  dds::with_key::datawriter as datawriter_with_key,
};

use super::wrappers::{NoKeyWrapper, SAWrapper};

/// DataWriter for no key topics
pub struct DataWriter<'a, D: Serialize, SA: SerializerAdapter<D> = CDRSerializerAdapter<D>> {
  keyed_datawriter: datawriter_with_key::DataWriter<'a, NoKeyWrapper<D>, SAWrapper<SA>>,
}

impl<'a, D, SA> DataWriter<'a, D, SA>
where
  D: Serialize,
  SA: SerializerAdapter<D>,
{
  pub(crate) fn from_keyed(
    keyed: datawriter_with_key::DataWriter<'a, NoKeyWrapper<D>, SAWrapper<SA>>,
  ) -> DataWriter<'a, D, SA> {
    DataWriter {
      keyed_datawriter: keyed,
    }
  }

  // write (with optional timestamp)
  pub fn write(&self, data: D, source_timestamp: Option<Timestamp>) -> Result<()> {
    self
      .keyed_datawriter
      .write(NoKeyWrapper::<D> { d: data }, source_timestamp)
  }

  pub fn wait_for_acknowledgments(&self, max_wait: Duration) -> Result<()> {
    self.keyed_datawriter.wait_for_acknowledgments(max_wait)
  }

  // status queries
  /// Unimplemented. <b>Do not use</b>.
  pub fn get_liveliness_lost_status(&self) -> Result<LivelinessLostStatus> {
    self.keyed_datawriter.get_liveliness_lost_status()
  }

  /// Should get latest offered deadline missed status. <b>Do not use yet</b> use `get_status_lister` instead for the moment.
  pub fn get_offered_deadline_missed_status(&self) -> Result<OfferedDeadlineMissedStatus> {
    self.keyed_datawriter.get_offered_deadline_missed_status()
  }

  /// Unimplemented. <b>Do not use</b>.
  pub fn get_offered_incompatible_qos_status(&self) -> Result<OfferedIncompatibleQosStatus> {
    self.keyed_datawriter.get_offered_incompatible_qos_status()
  }

  /// Unimplemented. <b>Do not use</b>.
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

  /// Unimplemented. <b>Do not use</b>.
  pub fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData> {
    self.keyed_datawriter.get_matched_subscriptions()
  }

  pub fn get_status_listener(&self) -> &Receiver<StatusChange> {
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
  use crate::dds::{participant::DomainParticipant, topic::TopicKind};
  use crate::test::random_data::*;
  use crate::serialization::cdr_serializer::*;
  use byteorder::LittleEndian;

  #[test]
  fn dw_write_test() {
    let domain_participant = DomainParticipant::new(0);
    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", "Huh?", &qos, TopicKind::NoKey)
      .expect("Failed to create topic");

    let data_writer: DataWriter<'_, RandomData, CDRSerializerAdapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter_no_key(None, &topic, None)
        .expect("Failed to create datawriter");

    let mut data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    data_writer
      .write(data.clone(), None)
      .expect("Unable to write data");

    data.a = 5;
    let timestamp: Timestamp = Timestamp::now();
    data_writer
      .write(data, Some(timestamp))
      .expect("Unable to write data with timestamp");

    // TODO: verify that data is sent/writtent correctly
    // TODO: write also with timestamp
  }

  #[test]
  fn dw_wait_for_ack_test() {
    let domain_participant = DomainParticipant::new(0);
    let qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", "Huh?", &qos, TopicKind::NoKey)
      .expect("Failed to create topic");

    let data_writer: DataWriter<'_, RandomData, CDRSerializerAdapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter_no_key(None, &topic, None)
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
