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
      // The reader is created before the datareader, hence initializing the
      // latest_instant to now should be fine. There should be no smaller instants
      // added by the reader.
      latest_instant: Instant::now(),
    }
  }

  // Gets all unseen cache_changes from the TopicCache. Deserializes
  // the serialized payload and stores the DataSamples (the actual data and the
  // samplestate) to local container, datasample_cache.
  fn get_datasamples_from_cache(&mut self) {
    let dds_cache = self.dds_cache.read().unwrap();
    let cache_changes = dds_cache.from_topic_get_changes_in_range(
      &self.my_topic.get_name().to_string(),
      &self.latest_instant,
      &Instant::now(),
    );

    self.latest_instant = *cache_changes.last().unwrap().0;

    for (_instant, cc) in cache_changes {
      let cc_data_value = cc.data_value.as_ref().unwrap().clone();
      let ser_data = cc_data_value.value;
      // data_object needs to be initialized
      let mut data_object: D = deserialize_from_little_endian(&ser_data).unwrap();

      let last_bit = (1 << 15) & cc_data_value.representation_identifier;
      if last_bit == 1 {
        data_object = deserialize_from_big_endian(&ser_data).unwrap();
      }
      // TODO: how do we get the source_timestamp here? Is it needed?
      // TODO: Keeping track of and assigning  generation rank, sample rank etc.
      let mut datasample = DataSample::new(Timestamp::TIME_INVALID, data_object);
      datasample.sample_info.instance_state = Self::change_kind_to_instance_state(&cc.kind);
      self.datasample_cache.add_datasample(datasample).unwrap();
    }
    self.data_available = false;
  }

  /// This operation accesses a collection of Data values from the DataReader.
  /// The function return references to the data.
  /// The size of the returned collection will be limited to the specified max_samples.
  /// The read_condition filters the samples accessed.
  /// If no matching samples exist, then an empty Vec will be returned, but that is not an error.
  /// In case a disposed sample is returned, it is indicated by InstanceState in SampleInfo and
  /// DataSample.value will be Err containig the key of the disposed sample.
  ///
  /// View state and sample state are maintained for each DataReader separately. Instance state is
  /// determined by global CacheChange objects concerning that instance.
  ///
  /// If the sample belongs to the most recent generation of the instance,
  /// it will also set the view_state of the instance to NotNew.
  ///
  /// This should cover DDS DataReader methods read, read_w_condition and
  /// read_next_sample.
  pub fn read(
    &mut self,
    max_samples: usize,            // maximum number of DataSamples to return.
    read_condition: ReadCondition, // use e.g. ReadCondition::any() or ReadCondition::not_read()
  ) -> Result<Vec<&DataSample<D>>> {
    if self.data_available {
      self.get_datasamples_from_cache();
    }
    let mut result = Vec::new();
    'outer: for (_, datasample_vec) in self.datasample_cache.datasamples.iter_mut() {
      for datasample in datasample_vec.iter_mut() {
        if Self::matches_conditions(&read_condition, datasample) {
          datasample.sample_info.sample_state = SampleState::Read;
          result.push(&*datasample);
        }
        if result.len() >= max_samples {
          break 'outer;
        }
      }
    }
    Ok(result)
  }

  /// Similar to read, but insted of references being returned, the datasamples
  /// are removed from the DataReader and ownership is transferred to the caller.
  /// Should cover take, take_w_condition and take_next_sample
  pub fn take(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<DataSample<D>>> {
    if self.data_available {
      self.get_datasamples_from_cache();
    }

    let mut result = Vec::new();
    'outer: for (_, datasample_vec) in self.datasample_cache.datasamples.iter_mut() {
      let mut ind = 0;
      while ind < datasample_vec.len() {
        // If a datasample is removed from the vec, all elements from the index
        // onwards will be shifted left. Therefore, the next sample is accessible
        // in the same index
        if Self::matches_conditions(&read_condition, &datasample_vec[ind]) {
          let mut datasample = datasample_vec.remove(ind);
          datasample.sample_info.sample_state = SampleState::Read;
          result.push(datasample);
        // Nothing removed, next element can be found in the next index.
        } else {
          ind += 1;
        }
        if result.len() >= max_samples {
          break 'outer;
        }
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
  /// This should cover DDS DataReader methods read_instance, read_next_instance,
  /// read_next_instance_w_condition.
  pub fn read_instance(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
    // Select only samples from instance specified by key. In case of None, select the
    // "smallest" instance as specified by the key type Ord trait.
    instance_key: Option<<D as Keyed>::K>,
    // This = Select instance specified by key.
    // Next = select next instance in the order specified by Ord on keys.
    this_or_next: SelectByKey,
  ) -> Result<Vec<&DataSample<D>>> {
    if self.data_available {
      self.get_datasamples_from_cache();
    }
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
    let mut result = Vec::new();
    let datasample_vec = self.datasample_cache.get_datasamples_mut(&key).unwrap();
    for datasample in datasample_vec.iter_mut() {
      if Self::matches_conditions(&read_condition, datasample) {
        datasample.sample_info.sample_state = SampleState::Read;
        result.push(&*datasample);
      }
      if result.len() >= max_samples {
        break;
      }
    }
    Ok(result)
  }

  /// Similar to read_instance, but will return owned datasamples
  /// This should cover DDS DataReader methods take_instance, take_next_instance,
  /// take_next_instance_w_condition.
  pub fn take_instance(
    &mut self,
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
    let mut result = Vec::new();

    let datasample_vec = self.datasample_cache.datasamples.get_mut(&key).unwrap();
    let mut ind = 0;
    while ind < datasample_vec.len() {
      // If a datasample is removed from the vec, all elements from the index
      // onwards will be shifted left. Therefore, the next sample is accessible
      // in the same index
      if Self::matches_conditions(&read_condition, &datasample_vec[ind]) {
        let mut datasample = datasample_vec.remove(ind);
        datasample.sample_info.sample_state = SampleState::Read;
        result.push(datasample);
      // Nothing removed, next element can be found in the next index.
      } else {
        ind += 1;
      }
      if result.len() >= max_samples {
        break;
      }
    }
    Ok(result)
  }

  /// This is a simplified API for reading the next not_read sample
  /// If no new data is available, the return value is Ok(None).
  pub fn read_next_sample(&mut self, _take: Take) -> Result<Option<&DataSample<D>>> {
    let mut ds = self.read(1, ReadCondition::not_read())?;
    Ok(ds.pop())
  }

  pub fn take_next_sample(&mut self, _take: Take) -> Result<Option<DataSample<D>>> {
    let mut ds = self.take(1, ReadCondition::not_read())?;
    Ok(ds.pop())
  }

  // Could be called when readers send notification.
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
  use crate::structure::sequence_number::SequenceNumber;

  use crate::serialization::cdrSerializer::to_little_endian_binary;

  use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
  #[test]
  fn dr_get_samples_from_ddschache() {
    let dp = DomainParticipant::new(10, 0);
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
    data.writer_sn = SequenceNumber::from(0);

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
      .value
      .as_ref()
      .unwrap();

    assert_eq!(deserialized_random_data, &random_data);

    // Test getting of next samples.
    let random_data2 = RandomData {
      a: 1,
      b: "somedata number 2".to_string(),
    };
    let mut data2 = Data::default();
    data2.reader_id = EntityId::createCustomEntityID([1, 2, 3], 111);
    data2.writer_id = writer_guid.entityId;
    data2.writer_sn = SequenceNumber::from(1);

    data2.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&random_data2).unwrap(),
    };

    let random_data3 = RandomData {
      a: 1,
      b: "third somedata".to_string(),
    };
    let mut data3 = Data::default();
    data3.reader_id = EntityId::createCustomEntityID([1, 2, 3], 111);
    data3.writer_id = writer_guid.entityId;
    data3.writer_sn = SequenceNumber::from(2);

    data3.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&random_data3).unwrap(),
    };

    new_reader.handle_data_msg(data2, mr_state.clone());
    new_reader.handle_data_msg(data3, mr_state);

    matching_datareader.get_datasamples_from_cache();
    let random_data_vec = matching_datareader
      .datasample_cache
      .get_datasample(&data_key)
      .unwrap();
    assert_eq!(random_data_vec.len(), 3);
  }

  #[test]
  fn dr_read_and_take() {
    let dp = DomainParticipant::new(11, 0);

    let mut qos = QosPolicies::qos_none();
    qos.history = Some(policy::History::KeepAll); // Just for testing

    let sub = dp.create_subscriber(&qos).unwrap();
    let topic = dp
      .create_topic("dr read", TypeDesc::new("read fn test?".to_string()), &qos)
      .unwrap();

    let (send, _rec) = mio_channel::sync_channel::<Instant>(10);

    let default_id = EntityId::default();
    let reader_guid = GUID::new_with_prefix_and_id(*dp.get_guid_prefix(), default_id);

    let mut reader = Reader::new(
      reader_guid,
      send,
      dp.get_dds_cache(),
      topic.get_name().to_string(),
    );

    let mut datareader = sub
      .create_datareader::<RandomData>(Some(default_id), &topic, &qos)
      .unwrap();

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };
    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;
    reader.matched_writer_add(writer_guid.clone(), mr_state.clone());

    // Reader and datareader ready, test with data
    let test_data = RandomData {
      a: 10,
      b: ":DDD".to_string(),
    };

    let test_data2 = RandomData {
      a: 11,
      b: ":)))".to_string(),
    };

    let mut data_msg = Data::default();
    data_msg.reader_id = *reader.get_entity_id();
    data_msg.writer_id = writer_guid.entityId;
    data_msg.writer_sn = SequenceNumber::from(0);

    data_msg.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&test_data).unwrap(),
    };

    let mut data_msg2 = Data::default();
    data_msg2.reader_id = *reader.get_entity_id();
    data_msg2.writer_id = writer_guid.entityId;
    data_msg2.writer_sn = SequenceNumber::from(1);

    data_msg2.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&test_data2).unwrap(),
    };
    reader.handle_data_msg(data_msg, mr_state.clone());
    reader.handle_data_msg(data_msg2, mr_state.clone());

    // Read the same sample two times.
    {
      let result_vec = datareader.read(100, ReadCondition::any()).unwrap();
      let d = result_vec[0].value.as_ref().unwrap();
      assert_eq!(&test_data, d);
    }
    {
      let result_vec2 = datareader.read(100, ReadCondition::any()).unwrap();
      let d2 = result_vec2[1].value.as_ref().unwrap();
      assert_eq!(&test_data2, d2);
    }
    {
      let result_vec3 = datareader.read(100, ReadCondition::any()).unwrap();
      let d3 = result_vec3[0].value.as_ref().unwrap();
      assert_eq!(&test_data, d3);
    }

    // Take
    let mut result_vec = datareader.take(100, ReadCondition::any()).unwrap();
    let result_vec2 = datareader.take(100, ReadCondition::any());

    let d2 = result_vec.pop().unwrap().value.unwrap();
    let d1 = result_vec.pop().unwrap().value.unwrap();
    assert_eq!(test_data2, d2);
    assert_eq!(test_data, d1);
    assert!(result_vec2.is_ok());
    assert_eq!(result_vec2.unwrap().len(), 0);

    //datareader.

    // Read and take tests with instant

    let data_key1 = RandomData {
      a: 1,
      b: ":D".to_string(),
    };
    let data_key2_1 = RandomData {
      a: 2,
      b: ":(".to_string(),
    };
    let data_key2_2 = RandomData {
      a: 2,
      b: ":)".to_string(),
    };
    let data_key2_3 = RandomData {
      a: 2,
      b: "xD".to_string(),
    };

    let key1 = data_key1.get_key();
    let key2 = data_key2_1.get_key();

    assert!(data_key2_1.get_key() == data_key2_2.get_key());
    assert!(data_key2_3.get_key() == key2);

    let mut data_msg = Data::default();
    data_msg.reader_id = *reader.get_entity_id();
    data_msg.writer_id = writer_guid.entityId;
    data_msg.writer_sn = SequenceNumber::from(2);

    data_msg.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&data_key1).unwrap(),
    };
    let mut data_msg2 = Data::default();
    data_msg2.reader_id = *reader.get_entity_id();
    data_msg2.writer_id = writer_guid.entityId;
    data_msg2.writer_sn = SequenceNumber::from(3);

    data_msg2.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&data_key2_1).unwrap(),
    };
    let mut data_msg3 = Data::default();
    data_msg3.reader_id = *reader.get_entity_id();
    data_msg3.writer_id = writer_guid.entityId;
    data_msg3.writer_sn = SequenceNumber::from(4);

    data_msg3.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&data_key2_2).unwrap(),
    };
    let mut data_msg4 = Data::default();
    data_msg4.reader_id = *reader.get_entity_id();
    data_msg4.writer_id = writer_guid.entityId;
    data_msg4.writer_sn = SequenceNumber::from(5);

    data_msg4.serialized_payload = SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: to_little_endian_binary(&data_key2_3).unwrap(),
    };
    reader.handle_data_msg(data_msg, mr_state.clone());
    reader.handle_data_msg(data_msg2, mr_state.clone());
    reader.handle_data_msg(data_msg3, mr_state.clone());
    reader.handle_data_msg(data_msg4, mr_state.clone());
    datareader.receive_notification();

    println!("calling read with key 1 and this");
    let results =
      datareader.read_instance(100, ReadCondition::any(), Some(key1), SelectByKey::This);
    assert_eq!(&data_key1, results.unwrap()[0].value.as_ref().unwrap());

    println!("calling read with None and this");
    // Takes the samllest key, 1 in this case.
    let results = datareader.read_instance(100, ReadCondition::any(), None, SelectByKey::This);
    assert_eq!(&data_key1, results.unwrap()[0].value.as_ref().unwrap());

    println!("calling read with key 1 and next");
    let results =
      datareader.read_instance(100, ReadCondition::any(), Some(key1), SelectByKey::Next);
    assert_eq!(results.as_ref().unwrap().len(), 3);
    assert_eq!(&data_key2_2, results.unwrap()[1].value.as_ref().unwrap());

    println!("calling take with key 2 and this");
    let results =
      datareader.take_instance(100, ReadCondition::any(), Some(key2), SelectByKey::This);
    assert_eq!(results.as_ref().unwrap().len(), 3);
    let mut vec = results.unwrap();
    let d3 = vec.pop().unwrap().value.unwrap();
    let d2 = vec.pop().unwrap().value.unwrap();
    let d1 = vec.pop().unwrap().value.unwrap();
    assert_eq!(data_key2_3, d3);
    assert_eq!(data_key2_2, d2);
    assert_eq!(data_key2_1, d1);

    println!("calling take with key 2 and this");
    let results =
      datareader.take_instance(100, ReadCondition::any(), Some(key2), SelectByKey::This);
    assert!(results.is_ok());
    assert!(results.unwrap().is_empty());
  }
}