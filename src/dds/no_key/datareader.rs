use std::io;

use serde::{de::DeserializeOwned};
use mio_extras::channel as mio_channel;
use mio::{Poll, Token, Ready, PollOpt, Evented};

use crate::{
  dds::datasample::DataSample,
  dds::interfaces::IDataReader,
  dds::interfaces::IDataSample,
  discovery::discovery::DiscoveryCommand,
  structure::{
    entity::{Entity, EntityAttributes},
    guid::{EntityId},
    dds_cache::DDSCache,
  },
};
use crate::dds::{
  traits::serde_adapters::*, values::result::*, qos::*, pubsub::Subscriber, topic::Topic,
  readcondition::*,
};

use crate::dds::datareader as datareader_with_key;

use std::sync::{Arc, RwLock};

use super::{
  wrappers::{NoKeyWrapper, SAWrapper},
};

// ----------------------------------------------------

// DataReader for NO_KEY data. Does not require "D: Keyed"
pub struct DataReader<'a, D, SA> {
  keyed_datareader: datareader_with_key::DataReader<'a, NoKeyWrapper<D>, SAWrapper<SA>>,
}

// TODO: rewrite DataSample so it can use current Keyed version (and send back datasamples instead of current data)
impl<'a, D: 'static, SA> DataReader<'a, D, SA>
where
  D: DeserializeOwned, // + Deserialize<'s>,
  SA: DeserializerAdapter<D>,
{
  pub fn new(
    subscriber: &'a Subscriber,
    my_id: EntityId,
    topic: &'a Topic,
    notification_receiver: mio_channel::Receiver<()>,
    dds_cache: Arc<RwLock<DDSCache>>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  ) -> Result<Self> {
    Ok(DataReader {
      keyed_datareader: datareader_with_key::DataReader::<'a, NoKeyWrapper<D>, SAWrapper<SA>>::new(
        subscriber,
        my_id,
        topic,
        notification_receiver,
        dds_cache,
        discovery_command,
      )?,
    })
  }

  pub fn from_keyed(
    keyed: datareader_with_key::DataReader<'a, NoKeyWrapper<D>, SAWrapper<SA>>,
  ) -> DataReader<'a, D, SA> {
    DataReader {
      keyed_datareader: keyed,
    }
  }
} // impl

impl<'a, D: 'static, SA> IDataReader<D, SA> for DataReader<'a, D, SA>
where
  D: DeserializeOwned, // + Deserialize<'s>,
  SA: DeserializerAdapter<D>,
{
  fn read(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<&dyn IDataSample<D>>> {
    let values: Vec<&dyn IDataSample<D>> = match self
      .keyed_datareader
      .read_as_obj(max_samples, read_condition)
    {
      Ok(v) => v
        .iter()
        .map(|p| <DataSample<NoKeyWrapper<D>> as IDataSample<D>>::as_idata_sample(p))
        .collect(),
      Err(e) => return Err(e),
    };
    Ok(values)
  }

  fn take(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<Box<dyn IDataSample<D>>>> {
    let values: Vec<Box<dyn IDataSample<D>>> = match self
      .keyed_datareader
      .take_as_obj(max_samples, read_condition)
    {
      Ok(v) => v
        .into_iter()
        .map(|p| <DataSample<NoKeyWrapper<D>> as IDataSample<D>>::into_idata_sample(p))
        .collect(),
      Err(e) => return Err(e),
    };

    Ok(values)
  }

  fn read_next_sample(&mut self) -> Result<Option<&dyn IDataSample<D>>> {
    let mut ds = self.read(1, ReadCondition::not_read())?;
    let val = match ds.pop() {
      Some(v) => Some(v.as_idata_sample()),
      None => None,
    };
    Ok(val)
  }

  fn take_next_sample(&mut self) -> Result<Option<Box<dyn IDataSample<D>>>> {
    let mut ds = self.take(1, ReadCondition::not_read())?;
    let val = match ds.pop() {
      Some(v) => Some(v),
      None => None,
    };
    Ok(val)
  }

  fn get_requested_deadline_missed_status() -> Result<RequestedDeadlineMissedStatus> {
    todo!()
  }
}

// This is  not part of DDS spec. We implement mio Eventd so that the application can asynchronously
// poll DataReader(s).
impl<'a, D, SA> Evented for DataReader<'a, D, SA> {
  // We just delegate all the operations to notification_receiver, since it alrady implements Evented
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self
      .keyed_datareader
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
      .keyed_datareader
      .notification_receiver
      .reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.keyed_datareader.notification_receiver.deregister(poll)
  }
}

impl<D, SA> HasQoSPolicy for DataReader<'_, D, SA> {
  fn set_qos(&mut self, policy: &QosPolicies) -> Result<()> {
    self.keyed_datareader.set_qos(policy)
  }

  fn get_qos(&self) -> &QosPolicies {
    self.keyed_datareader.get_qos()
  }
}

impl<'a, D, SA> Entity for DataReader<'a, D, SA>
where
  D: DeserializeOwned,
  SA: DeserializerAdapter<D>,
{
  fn as_entity(&self) -> &EntityAttributes {
    self.keyed_datareader.as_entity()
  }
}
