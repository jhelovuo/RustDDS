use std::{
  time::{Duration},
};

use serde::Serialize;

use crate::{serialization::CDRSerializerAdapter, structure::time::Timestamp};
use crate::structure::entity::{RTPSEntity};

use crate::dds::pubsub::Publisher;
use crate::dds::topic::Topic;
use crate::dds::values::result::Result;

use crate::dds::data_types::*;
use crate::dds::traits::dds_entity::DDSEntity;

use crate::dds::traits::serde_adapters::no_key::SerializerAdapter;

use crate::dds::qos::{HasQoSPolicy, QosPolicies};

use crate::{
  discovery::data_types::topic_data::SubscriptionBuiltinTopicData,
  dds::with_key::datawriter as datawriter_with_key,
};

use super::wrappers::{NoKeyWrapper, SAWrapper};

/// DDS DataWriter for no key topics
///
/// # Examples
///
/// ```
/// use serde::{Serialize, Deserialize};
/// use rustdds::dds::DomainParticipant;
/// use rustdds::dds::qos::QosPolicyBuilder;
/// use rustdds::dds::data_types::TopicKind;
/// use rustdds::dds::No_Key_DataWriter as DataWriter;
/// use rustdds::serialization::CDRSerializerAdapter;
///
/// let domain_participant = DomainParticipant::new(0).unwrap();
/// let qos = QosPolicyBuilder::new().build();
/// let publisher = domain_participant.create_publisher(&qos).unwrap();
///
/// #[derive(Serialize, Deserialize)]
/// struct SomeType {}
///
/// // NoKey is important
/// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
/// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None);
/// ```
pub struct DataWriter<D: Serialize, SA: SerializerAdapter<D> = CDRSerializerAdapter<D>> {
  keyed_datawriter: datawriter_with_key::DataWriter<NoKeyWrapper<D>, SAWrapper<SA>>,
}

impl<D, SA> DataWriter<D, SA>
where
  D: Serialize,
  SA: SerializerAdapter<D>,
{
  pub(crate) fn from_keyed(
    keyed: datawriter_with_key::DataWriter<NoKeyWrapper<D>, SAWrapper<SA>>,
  ) -> DataWriter<D, SA> {
    DataWriter {
      keyed_datawriter: keyed,
    }
  }

  /// Writes single data instance to a topic.
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// let some_data = SomeType {};
  /// data_writer.write(some_data, None).unwrap();
  /// ```
  pub fn write(&self, data: D, source_timestamp: Option<Timestamp>) -> Result<()> {
    self
      .keyed_datawriter
      .write(NoKeyWrapper::<D> { d: data }, source_timestamp)
  }

  /// Waits for all acknowledgements to finish
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use std::time::Duration;
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// data_writer.wait_for_acknowledgments(Duration::from_millis(100));
  /// ```
  pub fn wait_for_acknowledgments(&self, max_wait: Duration) -> Result<bool> {
    self.keyed_datawriter.wait_for_acknowledgments(max_wait)
  }
  /*
  // status queries
  /// Unimplemented. <b>Do not use</b>.
  ///
  /// # Examples
  ///
  // TODO: enable run when implemented
  /// ```no_run
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// if let Ok(status) = data_writer.get_liveliness_lost_status() {
  ///   // Do something
  /// }
  /// ```
  pub fn get_liveliness_lost_status(&self) -> Result<LivelinessLostStatus> {
    self.keyed_datawriter.get_liveliness_lost_status()
  }

  /// Should get latest offered deadline missed status. <b>Do not use yet</b> use `get_status_lister` instead for the moment.
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use std::time::Duration;
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// if let Ok(odl_status) = data_writer.get_offered_deadline_missed_status() {
  ///   // Do something
  /// }
  /// ```
  pub fn get_offered_deadline_missed_status(&self) -> Result<OfferedDeadlineMissedStatus> {
    self.keyed_datawriter.get_offered_deadline_missed_status()
  }

  /// Unimplemented. <b>Do not use</b>.
  ///
  /// # Examples
  ///
  // TODO: enable run when implemented
  /// ```no_run
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// if let Ok(status) = data_writer.get_offered_incompatible_qos_status() {
  ///   // Do something
  /// }
  /// ```
  pub fn get_offered_incompatible_qos_status(&self) -> Result<OfferedIncompatibleQosStatus> {
    self.keyed_datawriter.get_offered_incompatible_qos_status()
  }

  /// Unimplemented. <b>Do not use</b>.
  ///
  /// # Examples
  ///
  // TODO: enable run when implemented
  /// ```no_run
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// if let Ok(status) = data_writer.get_publication_matched_status() {
  ///   // Do something
  /// }
  /// ```
  pub fn get_publication_matched_status(&self) -> Result<PublicationMatchedStatus> {
    self.keyed_datawriter.get_publication_matched_status()
  }
  */
  /// Topic this DataWriter is connected to.
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic.clone(), None).unwrap();
  ///
  /// assert_eq!(&topic, data_writer.get_topic());
  /// ```
  pub fn get_topic(&self) -> &Topic {
    self.keyed_datawriter.get_topic()
  }

  /// Publisher this DataWriter is connected to.
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// assert_eq!(&publisher, data_writer.get_publisher());
  /// ```
  pub fn get_publisher(&self) -> &Publisher {
    self.keyed_datawriter.get_publisher()
  }

  /// Manually asserts liveliness if QoS agrees
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// data_writer.assert_liveliness();
  /// ```
  pub fn assert_liveliness(&self) -> Result<()> {
    self.keyed_datawriter.assert_liveliness()
  }

  /// Unimplemented. <b>Do not use</b>.
  ///
  /// # Examples
  ///
  // TODO: enable run when implemented
  /// ```no_run
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// for sub in data_writer.get_matched_subscriptions().iter() {
  ///   // handle subscriptions
  /// }
  /// ```
  pub fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData> {
    self.keyed_datawriter.get_matched_subscriptions()
  }
  /*
  /// Gets mio receiver for all implemented Status changes
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::dds::No_Key_DataWriter as DataWriter;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// // NoKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// // Some status has changed
  ///
  /// if let Ok(status) = data_writer.get_status_listener().try_recv() {
  ///   // handle status change
  /// }
  /// ```
  pub fn get_status_listener(&self) -> &Receiver<DataWriterStatus> {
    self.keyed_datawriter.get_status_listener()
  } */
}

impl<D: Serialize, SA: SerializerAdapter<D>> RTPSEntity for DataWriter<D, SA> {
  fn get_guid(&self) -> GUID {
    self.keyed_datawriter.get_guid()
  }
}

impl<D: Serialize, SA: SerializerAdapter<D>> HasQoSPolicy for DataWriter<D, SA> {
  // fn set_qos(&mut self, policy: &QosPolicies) -> Result<()> {
  //   self.keyed_datawriter.set_qos(policy)
  // }

  fn get_qos(&self) -> QosPolicies {
    self.keyed_datawriter.get_qos()
  }
}

impl<D: Serialize, SA: SerializerAdapter<D>> DDSEntity for DataWriter<D, SA> {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dds::{participant::DomainParticipant, topic::TopicKind};
  use crate::test::random_data::*;
  use crate::serialization::cdr_serializer::*;
  use byteorder::LittleEndian;

  #[test]
  fn dw_write_test() {
    let domain_participant = DomainParticipant::new(0).expect("Failed to create participant");
    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic(
        "Aasii".to_string(),
        "Huh?".to_string(),
        &qos,
        TopicKind::NoKey,
      )
      .expect("Failed to create topic");

    let data_writer: DataWriter<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter_no_key(topic, None)
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
    let domain_participant = DomainParticipant::new(0).expect("Failed to create participant");
    let qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic(
        "Aasii".to_string(),
        "Huh?".to_string(),
        &qos,
        TopicKind::NoKey,
      )
      .expect("Failed to create topic");

    let data_writer: DataWriter<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter_no_key(topic, None)
        .expect("Failed to create datawriter");

    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    data_writer.write(data, None).expect("Unable to write data");

    let res = data_writer
      .wait_for_acknowledgments(Duration::from_secs(2))
      .unwrap();
    assert!(res); // no "Reliable" QoS policy => "succeeds" immediately
  }
}
