use std::io;

use serde::{de::DeserializeOwned};
use mio::{Poll, Token, Ready, PollOpt, Evented};

use crate::{
  dds::datasample::DataSample,
  dds::interfaces::IDataReader,
  dds::interfaces::IDataSample,
  serialization::CDRDeserializerAdapter,
  dds::interfaces::IDataSampleConvert,
  structure::{
    entity::{Entity, EntityAttributes},
  },
};
use crate::dds::{traits::serde_adapters::*, values::result::*, qos::*, readcondition::*};

use crate::dds::datareader as datareader_with_key;

use super::{
  wrappers::{NoKeyWrapper, SAWrapper},
};

// ----------------------------------------------------

// DataReader for NO_KEY data. Does not require "D: Keyed"
/// DDS DataReader for no key topics.
pub struct DataReader<
  'a,
  D: DeserializeOwned,
  DA: DeserializerAdapter<D> = CDRDeserializerAdapter<D>,
> {
  keyed_datareader: datareader_with_key::DataReader<'a, NoKeyWrapper<D>, SAWrapper<DA>>,
}

// TODO: rewrite DataSample so it can use current Keyed version (and send back datasamples instead of current data)
impl<'a, D: 'static, DA> DataReader<'a, D, DA>
where
  D: DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  pub(crate) fn from_keyed(
    keyed: datareader_with_key::DataReader<'a, NoKeyWrapper<D>, SAWrapper<DA>>,
  ) -> DataReader<'a, D, DA> {
    DataReader {
      keyed_datareader: keyed,
    }
  }
} // impl

impl<'a, D: 'static, DA> IDataReader<D, DA> for DataReader<'a, D, DA>
where
  D: DeserializeOwned, // + Deserialize<'s>,
  DA: DeserializerAdapter<D>,
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
        .map(|p| <DataSample<NoKeyWrapper<D>> as IDataSampleConvert<D>>::as_idata_sample(p))
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
        .map(|p| <DataSample<NoKeyWrapper<D>> as IDataSampleConvert<D>>::into_idata_sample(p))
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

  fn get_requested_deadline_missed_status(&self) -> Result<RequestedDeadlineMissedStatus> {
    self.keyed_datareader.get_requested_deadline_missed_status()
  }
}

// This is  not part of DDS spec. We implement mio Eventd so that the application can asynchronously
// poll DataReader(s).
impl<'a, D, DA> Evented for DataReader<'a, D, DA>
where
  D: DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
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

impl<D, DA> HasQoSPolicy for DataReader<'_, D, DA>
where
  D: DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  fn set_qos(&mut self, policy: &QosPolicies) -> Result<()> {
    self.keyed_datareader.set_qos(policy)
  }

  fn get_qos(&self) -> &QosPolicies {
    self.keyed_datareader.get_qos()
  }
}

impl<'a, D, DA> Entity for DataReader<'a, D, DA>
where
  D: DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  fn as_entity(&self) -> &EntityAttributes {
    self.keyed_datareader.as_entity()
  }
}
