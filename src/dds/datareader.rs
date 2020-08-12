use serde::Deserialize;
use mio_extras::channel as mio_channel;
use mio::{Poll, Token, Ready, PollOpt, Evented};
use std::io;

use crate::dds::traits::key::*;

use crate::structure::{
  entity::{Entity, EntityAttributes},
  guid::GUID,
  //time::Timestamp,
};

use crate::dds::{
  values::result::*, qos::*, datasample::*, datasample_cache::DataSampleCache, /*ddsdata::DDSData,*/
  pubsub::Subscriber,
  readcondition::*,
};

use std::sync::{Arc, RwLock};
use crate::structure::guid::{EntityId};
use crate::structure::{dds_cache::DDSCache};
use crate::dds::topic::Topic;

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
  notification_receiver: mio_channel::Receiver<()>,

  dds_cache: Arc<RwLock<DDSCache>>,
  // Is this needed here??
  datasample_cache: DataSampleCache<D>,
}

// TODO: rewrite DataSample so it can use current Keyed version (and send back datasamples instead of current data)

impl<'s, 'a, D> DataReader<'a, D>
where
  D: Deserialize<'s> + Keyed,
  <D as Keyed>::K : Key,
{
  pub fn new(
    subscriber: &'a Subscriber,
    my_id: EntityId,
    topic: &'a Topic,
    notification_receiver: mio_channel::Receiver<()>,
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
    }
  }

  pub fn add_datasample(&self, _datasample: D) -> Result<()> {
    Ok(())
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
where D:Keyed 
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

impl<'a, D> Entity for DataReader<'a, D>
where
  D: Deserialize<'a> + Keyed,
{
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
  }
}
