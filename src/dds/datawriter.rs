use std::{
  sync::{Arc, RwLock},
  time::{Duration},
  marker::PhantomData,
};
use mio_extras::channel as mio_channel;

use serde::Serialize;
use log::{error, warn};

use crate::{structure::time::Timestamp, discovery::discovery::DiscoveryCommand};
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

use crate::dds::qos::{
  HasQoSPolicy, QosPolicies,
  policy::{Reliability},
};
use crate::dds::traits::serde_adapters::SerializerAdapter;
use crate::dds::datasample::DataSample;
use crate::{discovery::data_types::topic_data::SubscriptionBuiltinTopicData, dds::ddsdata::DDSData};
use super::datasample_cache::DataSampleCache;

pub struct DataWriter<'a, D: Keyed + Serialize, SA: SerializerAdapter<D>> {
  my_publisher: &'a Publisher,
  my_topic: &'a Topic,
  qos_policy: QosPolicies,
  entity_attributes: EntityAttributes,
  cc_upload: mio_channel::SyncSender<DDSData>,
  discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  dds_cache: Arc<RwLock<DDSCache>>,
  datasample_cache: DataSampleCache<D>,
  phantom: PhantomData<SA>,
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
  pub fn new(
    publisher: &'a Publisher,
    topic: &'a Topic,
    guid: Option<GUID>,
    cc_upload: mio_channel::SyncSender<DDSData>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
    dds_cache: Arc<RwLock<DDSCache>>,
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
        super::qos::policy::LivelinessKind::Automatic => (),
        super::qos::policy::LivelinessKind::ManualByParticipant => {
          match discovery_command.send(DiscoveryCommand::REFRESH_LAST_MANUAL_LIVELINESS) {
            Ok(_) => (),
            Err(e) => {
              error!("Failed to send DiscoveryCommand - Refresh. {:?}", e);
            }
          }
        }
        super::qos::policy::LivelinessKind::ManulByTopic => (),
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
    })
  }

  // write (with optional timestamp)
  // This operation could take also in InstanceHandle, if we would use them.
  // The _with_timestamp version is covered by the optional timestamp.
  pub fn write(&mut self, data: D, source_timestamp: Option<Timestamp>) -> Result<()> {
    let mut ddsdata = DDSData::from(&data, source_timestamp);
    // TODO key value should be unique always. This is not always unique.
    // If sample with same values is given then hash is same for both samples.
    // TODO FIX THIS
    ddsdata.value_key_hash = data.get_key().into_hash_key();

    let _data_sample = match source_timestamp {
      Some(t) => DataSample::new(t, data, self.get_guid()),
      None => DataSample::new(Timestamp::from(time::get_time()), data, self.get_guid()),
    };

    match self.cc_upload.try_send(ddsdata) {
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

  // dispose
  // The data item is given only for identification, i.e. extracting the key
  pub fn dispose(
    &mut self,
    key: <D as Keyed>::K,
    source_timestamp: Option<Timestamp>,
  ) -> Result<()> {
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

    match self.cc_upload.try_send(ddsdata) {
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
  pub fn get_offered_incompatible_qos_status(&self) -> Result<OfferedIncompatibleQosStatus> {
    unimplemented!()
  }
  pub fn get_publication_matched_status(&self) -> Result<PublicationMatchedStatus> {
    unimplemented!()
  }

  // who are we connected to?
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
          super::qos::policy::LivelinessKind::Automatic => (),
          super::qos::policy::LivelinessKind::ManualByParticipant => (),
          super::qos::policy::LivelinessKind::ManulByTopic => {
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

  // DDS spec returns an InstanceHandles pointing to a BuiltInTopic reader
  //  but we do not use those, so return the actual data instead.
  pub fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData> {
    unimplemented!()
  }
  // This one function provides both get_matched_subscrptions and get_matched_subscription_data
  // TODO: Maybe we could return references to the subscription data to avoid copying?
  // But then what if the result set changes while the application processes it?

  fn refresh_manual_liveliness(&self) {
    match self.get_qos().liveliness {
      Some(lv) => match lv.kind {
        super::qos::policy::LivelinessKind::Automatic => (),
        super::qos::policy::LivelinessKind::ManualByParticipant => {
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
        super::qos::policy::LivelinessKind::ManulByTopic => (),
      },
      None => (),
    };
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
  use crate::dds::typedesc::TypeDesc;
  use crate::test::random_data::*;
  use std::thread;
  use crate::dds::traits::key::Keyed;
  use crate::serialization::cdrSerializer::CDR_serializer_adapter;
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

    let mut data_writer: DataWriter<
      '_,
      RandomData,
      CDR_serializer_adapter<RandomData, LittleEndian>,
    > = publisher
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
    let domain_participant = DomainParticipant::new(0);
    let qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos)
      .expect("Failed to create topic");

    let mut data_writer: DataWriter<
      '_,
      RandomData,
      CDR_serializer_adapter<RandomData, LittleEndian>,
    > = publisher
      .create_datawriter(None, &topic, &qos)
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
      .create_topic("Aasii", TypeDesc::new("Huh?".to_string()), &qos)
      .expect("Failed to create topic");

    let mut data_writer: DataWriter<
      '_,
      RandomData,
      CDR_serializer_adapter<RandomData, LittleEndian>,
    > = publisher
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
