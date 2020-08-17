use std::io;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use serde::Deserialize;
use mio_extras::channel as mio_channel;
use mio::{Poll, Token, Ready, PollOpt, Evented};

use crate::structure::{
  entity::{Entity, EntityAttributes},
  guid::{GUID, EntityId},
  time::Timestamp,
  dds_cache::DDSCache,
  cache_change::ChangeKind,
};
use crate::dds::{
  traits::key::*, values::result::*, qos::*, datasample::*, datasample_cache::DataSampleCache,
  pubsub::Subscriber, topic::Topic, readcondition::*,
};

use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
use crate::serialization::cdrDeserializer::deserialize_from_big_endian;




/// Specifies if a read operation should "take" the data, i.e. make it unavailable in the Datareader
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Take {
  No,
  Yes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectByKey {
  This,
  Next,
}

pub struct DataReader<'a, D: Keyed> {
  my_subscriber: &'a Subscriber,
  my_topic: &'a Topic,
  qos_policy: QosPolicies,
  entity_attributes: EntityAttributes,
  pub(crate) notification_receiver: mio_channel::Receiver<Instant>,

  dds_cache: Arc<RwLock<DDSCache>>,

  datasample_cache: DataSampleCache<D>,
  data_available: bool,
  latest_instant: Instant,
}

impl<'s, 'a, D> DataReader<'a, D>
where
  D: Deserialize<'s> + Keyed,
  <D as Keyed>::K: Key,
{
  pub fn new(
    subscriber: &'a Subscriber,
    my_id: EntityId,
    topic: &'a Topic,
    notification_receiver: mio_channel::Receiver<Instant>,
    dds_cache: Arc<RwLock<DDSCache>>,
  ) -> Self {
    let entity_attributes = EntityAttributes::new(GUID::new_with_prefix_and_id(
      *subscriber.domain_participant.get_guid_prefix(),
      my_id,
    ));

    Self {
      my_subscriber: subscriber,
      my_topic: topic,
      qos_policy: topic.get_qos().clone(),
      entity_attributes,
      notification_receiver,
      dds_cache,
      datasample_cache: DataSampleCache::new(topic.get_qos().clone()),
      data_available: true,
      latest_instant: Instant::now(), // TODO FIX TO A SMALL NUMBER
    }
  }

  // Gets all cache_changes from the TopicCache. Deserializes
  // the serialized payload and stores the DataSamples (the actual data and the
  // samplestate) to local container, datasample_cache.
  fn get_datasamples_from_cache(&mut self) {
    let dds_cache = self.dds_cache.read().unwrap();
    let cache_changes = dds_cache.from_topic_get_all_changes(&self.my_topic.get_name().to_string());

    let last_instant = *cache_changes.last().unwrap().0;

    // TODO: How to make sure that samples that are read and taken are not reread
    // but at the same time samples that are kept are updated
    for (_instant, cc) in cache_changes {
      // A completely new sample. Add it to datasample_cache
      let cc_data_value = cc.data_value.as_ref().unwrap().clone();
      let ser_data = cc_data_value.value;
      // data_object needs to be initialized
      let mut data_object: D = deserialize_from_little_endian(ser_data.clone()).unwrap();

      // TODO: last_bit as 1 means which endianness?
      let last_bit = (1 << 15) & cc_data_value.representation_identifier;
      if last_bit == 1 {
        data_object = deserialize_from_big_endian(ser_data).unwrap();
      }
      // TODO: how do we get the time here? Is it needed?
      let mut datasample = DataSample::new(Timestamp::TIME_INVALID, data_object);
      datasample.sample_info.instance_state = Self::change_kind_to_instance_state(&cc.kind);
      self.datasample_cache.add_datasample(datasample).unwrap();
    }
    self.latest_instant = last_instant;
    self.data_available = false;
  }

  /// This operation accesses a collection of Data values from the DataReader.
  /// The size of the returned collection will be limited to the specified max_samples.
  /// The read_condition filters the samples accessed.
  /// If no matching samples exist, then an empty Vec will be returned, but that is not an error.
  /// In case a disposed sample is returned, it is indicated by InstanceState in SampleInfo and
  /// DataSample.value will be Err containig the key of the disposed sample.
  ///
  /// View state and sample state are maintained for each DataReader separately. Instance state is
  /// determined by global CacheChange objects concerning that instance.
  ///
  /// The parameter "take" specifies if the returned samples are removed from the DataReader.
  /// The returned samples change their state in DataReader from NotRead to Read after this operation.
  /// If the sample belongs to the most recent generation of the instance,
  /// it will also set the view_state of the instance to NotNew.
  ///
  /// This should cover DDS DataReader methods read, take, read_w_condition, take_w_condition,
  /// read_next_sample, take_next_sample.
  pub fn read(
    &mut self,
    take: Take,                    // Take::Yes ( = take) or Take::No ( = read)
    max_samples: usize,            // maximum number of DataSamples to return.
    read_condition: ReadCondition, // use e.g. ReadCondition::any() or ReadCondition::not_read()
  ) -> Result<Vec<DataSample<D>>> {
    if self.data_available {
      self.get_datasamples_from_cache();
    }
    let mut result = Vec::new();
    // TODO! Are we iterating in a correct direction?
    'outer: for (_, datasample_vec) in self.datasample_cache.datasamples.iter_mut() {
      for datasample in datasample_vec.iter_mut() {
        if Self::matches_conditions(&read_condition, datasample) {
          datasample.sample_info.sample_state = SampleState::Read;
          result.push(datasample.clone());
        }
        if result.len() >= max_samples {
          break 'outer;
        }
      }
    }
    // Remove all that are read.
    if take == Take::Yes {
      for (_, datsample_vec) in self.datasample_cache.datasamples.iter_mut() {
        datsample_vec.retain(|elem| !elem.sample_info.sample_state == SampleState::Read)
      }
    }
    Ok(result)
  }

  /// Works similarly to read(), but will return only samples from a specific instance.
  /// The instance is specified by an optional key. In case the key is not specified, the smallest
  /// (in key order) instance is selected.
  /// If a key is specified, then the parameter this_or_next specifies whether to access the instance
  /// with specified key or the following one, in key order.
  ///
  /// This should cover DDS DataReader methods read_instance, take_instance, read_next_instance,
  /// take_next_instance, read_next_instance_w_condition, take_next_instance_w_condition.
  pub fn read_instance(
    &mut self,
    take: Take,
    max_samples: usize,
    read_condition: ReadCondition,
    // Select only samples from instance specified by key. In case of None, select the
    // "smallest" instance as specified by the key type Ord trait.
    instance_key: Option<<D as Keyed>::K>,
    // This = Select instance specified by key.
    // Next = select next instance in the order specified by Ord on keys.
    this_or_next: SelectByKey,
  ) -> Result<Vec<DataSample<D>>> {
    if self.data_available {
      self.get_datasamples_from_cache();
    }
    let mut result = Vec::new();

    let key = match instance_key {
      Some(k) => match this_or_next {
        SelectByKey::This => k,
        SelectByKey::Next => self.datasample_cache.get_next_key(&k),
      },
      None => self
        .datasample_cache
        .datasamples
        .keys()
        .min()
        .unwrap()
        .clone(),
    };

    let datasample_vec = self.datasample_cache.datasamples.get_mut(&key).unwrap();
    // Rest is almost identical to read.. combine them?
    for datasample in datasample_vec.iter_mut() {
      if Self::matches_conditions(&read_condition, datasample) {
        datasample.sample_info.sample_state = SampleState::Read;
        result.push(datasample.clone());
      }
      if result.len() >= max_samples {
        break;
      }
    }
    if take == Take::Yes {
      datasample_vec.retain(|elem| !!elem.sample_info.sample_state == SampleState::Read);
    }
    Ok(result)
  }

  /// This is a simplified API for reading the next not_read sample
  /// If no new data is available, the return value is Ok(None).
  pub fn read_next_sample(&mut self, take: Take) -> Result<Option<DataSample<D>>> {
    let mut ds = self.read(take, 1, ReadCondition::not_read())?;
    Ok(ds.pop())
  }

  pub fn receive_notification(&mut self) {
    self.data_available = true;
  }

  // Helper functions
  fn matches_conditions(rcondition: &ReadCondition, dsample: &DataSample<D>) -> bool {
    if !rcondition
      .sample_state_mask
      .contains(dsample.sample_info.sample_state)
    {
      return false;
    }
    if !rcondition
      .view_state_mask
      .contains(dsample.sample_info.view_state)
    {
      return false;
    }
    if !rcondition
      .instance_state_mask
      .contains(dsample.sample_info.instance_state)
    {
      return false;
    }
    true
  }

  fn change_kind_to_instance_state(c_k: &ChangeKind) -> InstanceState {
    match c_k {
      ChangeKind::ALIVE => InstanceState::Alive,
      ChangeKind::NOT_ALIVE_DISPOSED => InstanceState::NotAlive_Disposed,
      // TODO check this..?
      ChangeKind::NOT_ALIVE_UNREGISTERED => InstanceState::NotAlive_NoWriters,
    }
  }
} // impl

// This is  not part of DDS spec. We implement mio Eventd so that the application can asynchronously
// poll DataReader(s).
impl<'a, D> Evented for DataReader<'a, D>
where
  D: Keyed,
{
  // We just delegate all the operations to notification_receiver, since it already implements Evented
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self
      .notification_receiver
      .register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &Poll,
    token: Token,
    interest: Ready,
    opts: PollOpt,
  ) -> io::Result<()> {
    self
      .notification_receiver
      .reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.notification_receiver.deregister(poll)
  }
}

// Todo when others attributes have PartialEq implemented

impl<D> HasQoSPolicy for DataReader<'_, D>
where
  D: Keyed,
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

impl<'a, D> Entity for DataReader<'a, D>
where
  D: Deserialize<'a> + Keyed,
{
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dds::participant::DomainParticipant;
  use crate::dds::typedesc::TypeDesc;
  use crate::test::random_data::*;
  use crate::dds::traits::key::Keyed;
  use mio_extras::channel as mio_channel;
  use crate::dds::reader::Reader;
  use crate::messages::submessages::data::Data;
  use crate::dds::message_receiver::*;
  use crate::structure::guid::GuidPrefix;

  use crate::serialization::cdrSerializer::to_little_endian_binary;
  use std::thread;

  use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
  #[test]
  fn dr_get_samples_from_ddschache() {
    let dp = DomainParticipant::new(0, 0);
    let mut qos = QosPolicies::qos_none();
    qos.history = Some(policy::History::KeepAll);

    let sub = dp.create_subscriber(&qos).unwrap();
    let topic = dp
      .create_topic("dr", TypeDesc::new("drtest?".to_string()), &qos)
      .unwrap();

    let (send, _rec) = mio_channel::sync_channel::<Instant>(10);

    let reader_id = EntityId::default();
    let datareader_id = EntityId::default();
    let reader_guid = GUID::new_with_prefix_and_id(*dp.get_guid_prefix(), reader_id);

    let mut new_reader = Reader::new(
      reader_guid,
      send,
      dp.get_dds_cache(),
      topic.get_name().to_string(),
    );

    let mut matching_datareader = sub
      .create_datareader::<RandomData>(Some(datareader_id), &topic, &qos)
      .unwrap();

    let random_data = RandomData {
      a: 1,
      b: "somedata".to_string(),
    };
    let data_key = random_data.get_key();

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };
    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;

    new_reader.matched_writer_add(writer_guid.clone(), mr_state.clone());

    let mut data = Data::default();
    data.reader_id = EntityId::createCustomEntityID([1, 2, 3], 111);
    data.writer_id = writer_guid.entityId;

    data.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&random_data).unwrap(),
    };
    new_reader.handle_data_msg(data, mr_state.clone());

    matching_datareader.get_datasamples_from_cache();
    let deserialized_random_data = matching_datareader
      .datasample_cache
      .get_datasample(&data_key)
      .unwrap()[0]
      .clone()
      .value
      .unwrap();

    assert_eq!(*deserialized_random_data, random_data);
    println!("All good until here?");

    // Test getting of next samples.
    let random_data2 = RandomData {
      a: 2,
      b: "somedata number 2".to_string(),
    };
    let mut data2 = Data::default();
    data2.reader_id = EntityId::createCustomEntityID([1, 2, 3], 111);
    data2.writer_id = writer_guid.entityId;

    data2.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&random_data2).unwrap(),
    };

    let random_data3 = RandomData {
      a: 3,
      b: "third somedata".to_string(),
    };
    let mut data3 = Data::default();
    data3.reader_id = EntityId::createCustomEntityID([1, 2, 3], 111);
    data3.writer_id = writer_guid.entityId;

    data3.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&random_data3).unwrap(),
    };

    new_reader.handle_data_msg(data2, mr_state.clone());
    new_reader.handle_data_msg(data3, mr_state);
    thread::sleep(time::Duration::milliseconds(1000).to_std().unwrap());

    matching_datareader.get_datasamples_from_cache();
    let random_data_vec = matching_datareader
      .datasample_cache
      .get_datasample(&data_key)
      .unwrap();
    assert_eq!(random_data_vec.len(), 2);
  }
}
