use std::io;

use serde::{de::DeserializeOwned};
use mio::{Poll, Token, Ready, PollOpt, Evented};

use crate::{
  serialization::CDRDeserializerAdapter,
  structure::{
    entity::{Entity, EntityAttributes},
  },
};
use crate::dds::{traits::serde_adapters::*, values::result::*, qos::*, readcondition::*};

use crate::dds::with_key::datareader as datareader_with_key;
use crate::dds::with_key::datasample::DataSample as WithKeyDataSample;

use crate::dds::no_key::datasample::DataSample;

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

  pub fn read(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<DataSample<&D>>> {
    let values: Vec<WithKeyDataSample<&NoKeyWrapper<D>>> =
      self.keyed_datareader.read(max_samples, read_condition)?;
    let mut result = Vec::with_capacity(values.len());
    for ks in values {
      if let Some(s) = DataSample::<D>::from_with_key_ref(ks) {
        result.push(s)
      }
    }
    Ok(result)
  }

  pub fn take(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<DataSample<D>>> {
    let values: Vec<WithKeyDataSample<NoKeyWrapper<D>>> =
      self.keyed_datareader.take(max_samples, read_condition)?;
    let mut result = Vec::with_capacity(values.len());
    for ks in values {
      if let Some(s) = DataSample::<D>::from_with_key(ks) {
        result.push(s)
      }
    }
    Ok(result)
  }

  pub fn read_next_sample(&mut self) -> Result<Option<DataSample<&D>>> {
    let mut ds = self.read(1, ReadCondition::not_read())?;
    Ok(ds.pop())
  }

  pub fn take_next_sample(&mut self) -> Result<Option<DataSample<D>>> {
    let mut ds = self.take(1, ReadCondition::not_read())?;
    Ok(ds.pop())
  }

  // Iterator interface

  /// Produces an interator over the currently available NOT_READ samples.
  /// Yields only payload data, not SampleInfo metadata
  /// This is not called `iter()` because it takes a mutable reference to self.
  pub fn iterator(&mut self) -> Result<impl Iterator<Item = &D>> {
    // TODO: We could come up with a more efficent implementation than wrapping a read call
    Ok(
      self
        .read(std::usize::MAX, ReadCondition::not_read())?
        .into_iter()
        .map(|ds| ds.value),
    )
  }

  /// Produces an interator over the samples filtered b ygiven condition.
  /// Yields only payload data, not SampleInfo metadata
  pub fn conditional_iterator(
    &mut self,
    read_condition: ReadCondition,
  ) -> Result<impl Iterator<Item = &D>> {
    // TODO: We could come up with a more efficent implementation than wrapping a read call
    Ok(
      self
        .read(std::usize::MAX, read_condition)?
        .into_iter()
        .map(|ds| ds.value),
    )
  }

  /// Produces an interator over the currently available NOT_READ samples.
  /// Yields only payload data, not SampleInfo metadata
  /// Removes samples from `DataReader`.
  /// <strong>Note!</strong> If the iterator is only partially consumed, all the samples it could have provided
  /// are still removed from the `Datareader`.
  pub fn into_iterator(&mut self) -> Result<impl Iterator<Item = D>> {
    // TODO: We could come up with a more efficent implementation than wrapping a read call
    Ok(
      self
        .take(std::usize::MAX, ReadCondition::not_read())?
        .into_iter()
        .map(|ds| ds.value),
    )
  }

  /// Produces an interator over the samples filtered b ygiven condition.
  /// Yields only payload data, not SampleInfo metadata
  /// <strong>Note!</strong> If the iterator is only partially consumed, all the samples it could have provided
  /// are still removed from the `Datareader`.
  pub fn into_conditional_iterator(
    &mut self,
    read_condition: ReadCondition,
  ) -> Result<impl Iterator<Item = D>> {
    // TODO: We could come up with a more efficent implementation than wrapping a read call
    Ok(
      self
        .take(std::usize::MAX, read_condition)?
        .into_iter()
        .map(|ds| ds.value),
    )
  }

  pub fn get_requested_deadline_missed_status(&self) -> Result<RequestedDeadlineMissedStatus> {
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
