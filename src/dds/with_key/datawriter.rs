use std::{
  marker::PhantomData,
  sync::{Arc, RwLock},
  time::Duration,
};
use mio_extras::channel::{self as mio_channel, Receiver};

use serde::Serialize;
use log::{error, warn};

use crate::{
  serialization::CDRSerializerAdapter, discovery::discovery::DiscoveryCommand,
  structure::time::Timestamp,
};
use crate::structure::entity::{Entity, EntityAttributes};
use crate::structure::{
  dds_cache::DDSCache,
  guid::{GUID, EntityId},
  topic_kind::TopicKind,
};

use crate::dds::pubsub::Publisher;
use crate::dds::topic::Topic;
use crate::dds::values::result::{
  Result, Error, LivelinessLostStatus, OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus,
  PublicationMatchedStatus,
};
use crate::dds::traits::dds_entity::DDSEntity;
use crate::dds::traits::key::*;
use crate::dds::traits::TopicDescription;

use crate::dds::qos::{
  HasQoSPolicy, QosPolicies,
  policy::{Reliability},
  policy::LivelinessKind,
};
use crate::dds::traits::serde_adapters::SerializerAdapter;
use crate::dds::with_key::datasample::DataSample;
use crate::{discovery::data_types::topic_data::SubscriptionBuiltinTopicData, dds::ddsdata::DDSData};
use super::super::{
  datasample_cache::DataSampleCache, 
  values::result::StatusChange, writer::WriterCommand,
};

/// DDS DataWriter for keyed topics
pub struct DataWriter<'a, D: Keyed + Serialize, SA: SerializerAdapter<D> = CDRSerializerAdapter<D>>
{
  my_publisher: &'a Publisher,
  my_topic: &'a Topic,
  qos_policy: QosPolicies,
  entity_attributes: EntityAttributes,
  cc_upload: mio_channel::SyncSender<WriterCommand>,
  discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  dds_cache: Arc<RwLock<DDSCache>>,
  datasample_cache: DataSampleCache<D>,
  phantom: PhantomData<SA>,
  status_receiver: Receiver<StatusChange>,
}

impl<'a, D, SA> Drop for DataWriter<'a, D, SA>
where
  D: Keyed + Serialize,
  SA: SerializerAdapter<D>,
{
  fn drop(&mut self) {
    match self
      .discovery_command
      .send(DiscoveryCommand::REMOVE_LOCAL_WRITER {
        guid: self.get_guid(),
      }) {
      Ok(_) => {}
      Err(e) => error!(
        "Failed to send REMOVE_LOCAL_WRITER DiscoveryCommand. {:?}",
        e
      ),
    }
  }
}

impl<'a, D, SA> DataWriter<'a, D, SA>
where
  D: Keyed + Serialize,
  <D as Keyed>::K: Key,
  SA: SerializerAdapter<D>,
{
  pub(crate) fn new(
    publisher: &'a Publisher,
    topic: &'a Topic,
    guid: Option<GUID>,
    cc_upload: mio_channel::SyncSender<WriterCommand>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
    dds_cache: Arc<RwLock<DDSCache>>,
    status_receiver: Receiver<StatusChange>,
  ) -> Result<DataWriter<'a, D, SA>> {
    let entity_id = match guid {
      Some(g) => g.entityId.clone(),
      None => EntityId::ENTITYID_UNKNOWN,
    };

    let dp = match publisher.get_participant() {
      Some(dp) => dp,
      None => {
        error!("Cannot create new DataWriter, DomainParticipant doesn't exist.");
        return Err(Error::PreconditionNotMet);
      }
    };

    let entity_attributes = EntityAttributes::new(GUID::new_with_prefix_and_id(
      dp.get_guid_prefix().clone(),
      entity_id,
    ));

    match dds_cache.write() {
      Ok(mut cache) => cache.add_new_topic(
        &String::from(topic.get_name()),
        TopicKind::NO_KEY,
        topic.get_type(),
      ),
      Err(e) => panic!("DDSCache is poisoned. {:?}", e),
    };

    match topic.get_qos().liveliness {
      Some(lv) => match lv.kind {
        LivelinessKind::Automatic => (),
        LivelinessKind::ManualByParticipant => {
          match discovery_command.send(DiscoveryCommand::REFRESH_LAST_MANUAL_LIVELINESS) {
            Ok(_) => (),
            Err(e) => {
              error!("Failed to send DiscoveryCommand - Refresh. {:?}", e);
            }
          }
        }
        LivelinessKind::ManulByTopic => (),
      },
      None => (),
    };

    Ok(DataWriter {
      my_publisher: publisher,
      my_topic: topic,
      qos_policy: topic.get_qos().clone(),
      entity_attributes,
      cc_upload,
      discovery_command,
      dds_cache,
      datasample_cache: DataSampleCache::new(topic.get_qos().clone()),
      phantom: PhantomData,
      status_receiver,
    })
  }

  // This one function provides both get_matched_subscrptions and get_matched_subscription_data
  // TODO: Maybe we could return references to the subscription data to avoid copying?
  // But then what if the result set changes while the application processes it?

  pub fn refresh_manual_liveliness(&self) {
    match self.get_qos().liveliness {
      Some(lv) => match lv.kind {
        LivelinessKind::Automatic => (),
        LivelinessKind::ManualByParticipant => {
          match self
            .discovery_command
            .send(DiscoveryCommand::REFRESH_LAST_MANUAL_LIVELINESS)
          {
            Ok(_) => (),
            Err(e) => {
              error!("Failed to send DiscoveryCommand - Refresh. {:?}", e);
            }
          }
        }
        LivelinessKind::ManulByTopic => (),
      },
      None => (),
    };
  }


  pub fn write(&self, data: D, source_timestamp: Option<Timestamp>) -> Result<()> {
    let mut ddsdata = DDSData::from(&data, source_timestamp);
    // TODO key value should be unique always. This is not always unique.
    // If sample with same values is given then hash is same for both samples.
    // TODO FIX THIS
    ddsdata.value_key_hash = data.get_key().into_hash_key();

    let _data_sample = match source_timestamp {
      Some(t) => DataSample::new(t, data, self.get_guid()),
      None => DataSample::new(Timestamp::from(time::get_time()), data, self.get_guid()),
    };

    match self
      .cc_upload
      .try_send(WriterCommand::DDSData { data: ddsdata })
    {
      Ok(_) => {
        self.refresh_manual_liveliness();
        Ok(())
      }
      Err(e) => {
        warn!("Failed to write new data. {:?}", e);
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

  pub fn get_status_listener(&self) -> &Receiver<StatusChange> {
    match self
      .cc_upload
      .try_send(WriterCommand::ResetOfferedDeadlineMissedStatus {
        writer_guid: self.get_guid(),
      }) {
      Ok(_) => (),
      Err(e) => error!("Unable to send ResetOfferedDeadlineMissedStatus. {:?}", e),
    };
    &self.status_receiver
  }

  /// Unimplemented. <b>Do not use</b>.
  pub fn get_liveliness_lost_status(&self) -> Result<LivelinessLostStatus> {
    todo!()
  }

  /// Should get latest offered deadline missed status. <b>Do not use yet</b> use `get_status_lister` instead for the moment.
  pub fn get_offered_deadline_missed_status(&self) -> Result<OfferedDeadlineMissedStatus> {
    let mut fstatus = OfferedDeadlineMissedStatus::new();
    while let Ok(status) = self.status_receiver.try_recv() {
      match status {
        StatusChange::OfferedDeadlineMissedStatus { status } => fstatus = status,
        // TODO: possibly save old statuses
        _ => (),
      }
    }

    match self
      .cc_upload
      .try_send(WriterCommand::ResetOfferedDeadlineMissedStatus {
        writer_guid: self.get_guid(),
      }) {
      Ok(_) => (),
      Err(e) => error!("Unable to send ResetOfferedDeadlineMissedStatus. {:?}", e),
    };

    Ok(fstatus)
  }

  /// Unimplemented. <b>Do not use</b>.
  pub fn get_offered_incompatible_qos_status(&self) -> Result<OfferedIncompatibleQosStatus> {
    todo!()
  }

  /// Unimplemented. <b>Do not use</b>.
  pub fn get_publication_matched_status(&self) -> Result<PublicationMatchedStatus> {
    todo!()
  }

  pub fn get_topic(&self) -> &Topic {
    &self.my_topic
  }

  pub fn get_publisher(&self) -> &Publisher {
    &self.my_publisher
  }

  pub fn assert_liveliness(&self) -> Result<()> {
    self.refresh_manual_liveliness();

    match self.get_qos().liveliness {
      Some(lv) => {
        match lv.kind {
          LivelinessKind::Automatic => (),
          LivelinessKind::ManualByParticipant => (),
          LivelinessKind::ManulByTopic => {
            match self
              .discovery_command
              .send(DiscoveryCommand::ASSERT_TOPIC_LIVELINESS {
                writer_guid: self.get_guid(),
              }) {
              Ok(_) => (),
              Err(e) => {
                error!(
                  "Failed to send DiscoveryCommand - AssertLiveliness. {:?}",
                  e
                );
              }
            }
          }
        };
      }
      None => (),
    };

    Ok(())
  }

  /// Unimplemented. <b>Do not use</b>.
  pub fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData> {
    todo!()
  }

  /// Disposes data instance with specified key
  ///
  /// # Arguments
  ///
  /// * `key` - Key of the instance
  /// * `source_timestamp` - DDS source timestamp (None uses now as time as specified in DDS spec)
  pub fn dispose(&mut self, key: <D as Keyed>::K, source_timestamp: Option<Timestamp>) -> Result<()> {
    /*

    Removing this for now, as there is need to redesign the mechanism of transmitting dispose actions
    to RTPS Writer. Then the RTPS writer needs to execute the dispose. That is done by sending a DATA
    submessage with the serialized key instead of data, and sending inline QoS parameter
    StatusInfo_t (see RTPS spec 9.6.3.4) to indicate "disposed"
    */

    let mut ddsdata = DDSData::from_dispose::<D>(key.clone(), source_timestamp);
    // TODO key value should be unique always. This is not always unique.
    // If sample with same values is given then hash is same for both samples.
    // TODO FIX THIS
    ddsdata.value_key_hash = key.into_hash_key();

    // What does this block of code do? What is the purpose of _data_sample?
    let _data_sample: DataSample<D> = match source_timestamp {
      Some(t) => DataSample::<D>::new_disposed::<<D as Keyed>::K>(t, key, self.get_guid()),
      None => DataSample::new_disposed::<<D as Keyed>::K>(
        Timestamp::from(time::get_time()),
        key,
        self.get_guid(),
      ),
    };

    match self
      .cc_upload
      .try_send(WriterCommand::DDSData { data: ddsdata })
    {
      Ok(_) => {
        self.refresh_manual_liveliness();
        Ok(())
      }
      Err(huh) => {
        warn!("Error: {:?}", huh);
        Err(Error::OutOfResources)
      }
    }
  }
}

impl<D, SA> Entity for DataWriter<'_, D, SA>
where
  D: Keyed + Serialize,
  SA: SerializerAdapter<D>,
{
  fn as_entity(&self) -> &crate::structure::entity::EntityAttributes {
    &self.entity_attributes
  }
}

impl<D, SA> HasQoSPolicy for DataWriter<'_, D, SA>
where
  D: Keyed + Serialize,
  SA: SerializerAdapter<D>,
{
  fn set_qos(&mut self, policy: &QosPolicies) -> Result<()> {
    // TODO: check liveliness of qos_policy
    self.qos_policy = policy.clone();
    Ok(())
  }

  fn get_qos(&self) -> &QosPolicies {
    &self.qos_policy
  }
}

impl<D, SA> DDSEntity for DataWriter<'_, D, SA>
where
  D: Keyed + Serialize,
  SA: SerializerAdapter<D>,
{
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dds::participant::DomainParticipant;
  use crate::test::random_data::*;
  use std::thread;
  use crate::dds::traits::key::Keyed;
  use crate::serialization::cdr_serializer::CDRSerializerAdapter;
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
      .create_topic("Aasii", "Huh?", &qos)
      .expect("Failed to create topic");

    let data_writer: DataWriter<'_, RandomData, CDRSerializerAdapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter(None, &topic, None)
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
      .create_topic("Aasii", "Huh?", &qos)
      .expect("Failed to create topic");

    let mut data_writer: DataWriter<
      '_,
      RandomData,
      CDRSerializerAdapter<RandomData, LittleEndian>,
    > = publisher
      .create_datawriter(None, &topic, None)
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

    thread::sleep(time::Duration::milliseconds(100).to_std().unwrap());
    data_writer
      .dispose(data.get_key(), None)
      .expect("Unable to dispose data");

    // TODO: verify that dispose is sent correctly
  }

  #[test]
  fn dw_wait_for_ack_test() {
    let domain_participant = DomainParticipant::new(0);
    let qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", "Huh?", &qos)
      .expect("Failed to create topic");

    let data_writer: DataWriter<'_, RandomData, CDRSerializerAdapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter(None, &topic, None)
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
