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
};

use std::sync::{Arc, RwLock};
use crate::structure::guid::{EntityId};
use crate::structure::{dds_cache::DDSCache};
use crate::dds::topic::Topic;

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
  /// The different _mask parameters filter the samples accessed. If a mask is None, then there is
  /// no filtering. If it is e.g. Some(SampleState::NotRead), then only samples having the specified
  ///  sample state are returned. 
  /// If no matching samples exist, then an empty Vec will be returned, but that is not an error.
  /// In case a disposed sample is returned, it is indicated by InstanceState in SampleInfo and
  /// DataSample.value will be Err containig the key of the disposed sample.
  ///
  /// View state and sample state are maintained for each DataReader separately. Instance state is
  /// determined by global CacheChange objects concerning that instance.
  pub fn read(
    &self,
    _max_samples: usize,
    _sample_state_mask: Option<SampleState>,
    _view_state_mask: Option<ViewState>,
    _instance_state_mask: Option<InstanceState>,
  ) -> Result<Vec<DataSample<D>>> {
    unimplemented!();
    // Go through the historycache list and return all relevant in a vec.
  }

  /// Similar to read() above.
  ///
  /// The act of taking a sample removes it from the DataReader so it cannot be ‘read’ or ‘taken’ again
  /// by the same DataReader. In addition, the sample will change its view_state to NOT_NEW.
  /// Note that this applies only to samples that are returned as a result of the call.
  pub fn take(
    &self,
    _max_samples: usize,
    _sample_state_mask: Option<SampleState>,
    _view_state_mask: Option<ViewState>,
    _instance_state_mask: Option<InstanceState>,
  ) -> Result<Vec<DataSample<D>>> {
    unimplemented!()
  }

  /// This is a simplified API for .read( 1 , Some(SampleState::NotRead) , None, None) 
  /// If no new data is available, the return value is Ok(None).
  pub fn read_next(&self) -> Result<Option<DataSample<D>>> {
    todo!()
  }


  //TODO: The input parameter list may be horribly wrong. Do not implement before checking.
  pub fn read_instance(&self, _instance_key: <D as Keyed>::K) -> Result<Vec<D>> {
    todo!()
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
