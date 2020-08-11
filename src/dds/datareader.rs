use serde::Deserialize;
use mio_extras::channel as mio_channel;

use crate::structure::{
  instance_handle::InstanceHandle,
  entity::{Entity, EntityAttributes},
  guid::GUID,
  time::Timestamp,
};

use crate::dds::{
  values::result::*, qos::*, datasample::*, datasample_cache::DataSampleCache, ddsdata::DDSData,
  traits::datasample_trait::DataSampleTrait, pubsub::Subscriber,
};

pub struct DataReader<'a, D> {
  subscriber: &'a Subscriber,
  qos_policy: QosPolicies,
  entity_attributes: EntityAttributes,
  datasample_cache: DataSampleCache<D>,
  notification_receiver: mio_channel::Receiver<(DDSData, Timestamp)>,
  // TODO: rest of fields
}

// TODO: rewrite DataSample so it can use current Keyed version (and send back datasamples instead of current data)

impl<'s, 'a, D> DataReader<'a, D>
where
  D: Deserialize<'s> + DataSampleTrait,
{
  pub fn new(
    guid: &GUID,
    subscriber: &'a Subscriber,
    qos: &QosPolicies,
    new_data_receiver: mio_channel::Receiver<(DDSData, Timestamp)>,
  ) -> Self {
    Self {
      subscriber,
      qos_policy: qos.clone(),
      entity_attributes: EntityAttributes::new(guid.clone()), // todo
      datasample_cache: DataSampleCache::new(qos.clone()),
      notification_receiver: new_data_receiver,
    }
  }

  pub fn add_datasample(&self, _datasample: D) -> Result<()> {
    Ok(())
  }

  pub fn read(
    &self,
    _max_samples: i32,
    _sample_state: SampleState,
    _view_state: ViewState,
    _instance_state: InstanceState,
  ) -> Result<Vec<D>> {
    unimplemented!();
    // Go through the historycache list and return all relevant in a vec.
  }

  pub fn take(
    &self,
    _max_samples: i32,
    _sample_state: SampleState,
    _view_state: ViewState,
    _instance_state: InstanceState,
  ) -> Result<Vec<D>> {
    unimplemented!()
  }

  pub fn read_next(&self) -> Result<Vec<D>> {
    todo!()
  }

  pub fn read_instance(&self, _instance_handle: InstanceHandle) -> Result<Vec<D>> {
    todo!()
  }
} // impl

// This is  not part of DDS spec. We implement mio Eventd so that the application can asynchronously
// poll DataReader(s).
impl<'a,D> Evented for DataReader<'a,D> 
{
  // We just delegate all the operations to notification_receiver, since it alrady implements Evented
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
        -> io::Result<()>
  {
      self.notification_receiver.register(poll, token, interest, opts)
  }

  fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
      -> io::Result<()>
  {
      self.notification_receiver.reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
      self.notification_receiver.deregister(poll)
  }
}

impl<'a,D> Entity for DataReader<'a,D> 
  where
    D: Deserialize<'a> + DataSampleTrait,
{
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
  }
}
