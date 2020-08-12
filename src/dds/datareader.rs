use serde::Deserialize;
use mio_extras::channel as mio_channel;
use mio::{Poll, Token, Ready, PollOpt, Evented};
use std::io;
use crate::structure::{
  entity::{Entity, EntityAttributes},
  guid::{GUID, EntityId},
  time::Timestamp,
  dds_cache::DDSCache,
};
use crate::dds::{
  traits::key::*, values::result::*, qos::*, datasample::*, datasample_cache::DataSampleCache,
   pubsub::Subscriber, topic::Topic,
};

use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
use crate::serialization::cdrDeserializer::deserialize_from_big_endian;

use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Specifies if a read operation should "take" the data, i.e. make it unavailable in the Datareader
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Take {
  No,
  Yes,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SelectByKey {
  This,
  Next,
}

pub struct DataReader<'a, D:Keyed> {
  my_subscriber: &'a Subscriber,
  my_topic: &'a Topic,
  qos_policy: QosPolicies,
  entity_attributes: EntityAttributes,
  notification_receiver: mio_channel::Receiver<Instant>,

  dds_cache: Arc<RwLock<DDSCache>>,

  datasample_cache: DataSampleCache<D>,
  //sampleinfo_map: BTreeMap<Instant, SampleInfo>,
  latest_instant: Instant,
}

// TODO: rewrite DataSample so it can use current Keyed version (and send back datasamples instead of current data)
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
      //sampleinfo_map: BTreeMap::new(),
      latest_instant: Instant::now(), // TODO FIX TO A SMALL NUMBER
    }
  }

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
      
      let last_bit = (1 << 15) & cc_data_value.representation_identifier;
      let ser_data = cc_data_value.value;

      let mut data_object: D = deserialize_from_little_endian(ser_data.clone()).unwrap();

      if last_bit == 1 {
        data_object = deserialize_from_big_endian(ser_data).unwrap();
      } 

      // TODO: how do we get the time?
      let data_sample = DataSample::new(Timestamp::TIME_INVALID, data_object);
      self.datasample_cache.add_datasample(data_sample).unwrap();
    }
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
  /// rad_next_sample, take_next_sample.
  pub fn read(
    &self,
    _take: Take, // Take::Yes ( = take) or Take::No ( = read)
    _max_samples: usize, // maximum number of DataSamples to return.
    _read_condition: ReadCondition,  // use e.g. ReadCondition::any() or ReadCondition::not_read()
  ) -> Result<Vec<DataSample<D>>> {
    unimplemented!();
    // Go through the historycache list and return all relevant in a vec.
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
    &self,
    _take: Take,
    _max_samples: usize,
    _read_condition: ReadCondition,
    // Select only samples from instance specified by key. In case of None, select the
    // "smallest" instance as specified by the key type Ord trait.
    _instance_key: Option<<D as Keyed>::K>,
    // This = Select instance specified by key. 
    // Next = select next instance in the order specified by Ord on keys.
    _this_or_next: SelectByKey,
  ) -> Result<Vec<DataSample<D>>> {
    todo!()
  }

  /// This is a simplified API for reading the next not_read sample
  /// If no new data is available, the return value is Ok(None).
  pub fn read_next_sample(&self, take: Take) -> Result<Option<DataSample<D>>> {
    let mut ds = self.read(take,1,ReadCondition::not_read())?;
    Ok(ds.pop())
  }
} // impl

// This is  not part of DDS spec. We implement mio Eventd so that the application can asynchronously
// poll DataReader(s).
impl<'a, D> Evented for DataReader<'a, D>
where
  D: Keyed,
{
  // We just delegate all the operations to notification_receiver, since it alrady implements Evented
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
impl<D> PartialEq for DataReader<'_, D>
where
  D: Keyed,
{
  fn eq(&self, other: &Self) -> bool {
    //self.my_subscriber.get_ == other.my_subscriber &&
    self.my_topic.get_name() == other.my_topic.get_name() &&
    // self.qos_policy == other.qos_policy &&
    self.entity_attributes == other.entity_attributes
    // self.dds_cache == other.dds_cache &&
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
