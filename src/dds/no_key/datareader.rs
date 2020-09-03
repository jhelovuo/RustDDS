use std::io;
use std::ops::Deref;

use serde::{Deserialize, de::DeserializeOwned, Deserializer};
use mio_extras::channel as mio_channel;
use mio::{Poll, Token, Ready, PollOpt, Evented};

use crate::structure::{
  entity::{Entity, EntityAttributes},
  guid::{EntityId},
  dds_cache::DDSCache,
};
use crate::dds::{
  traits::key::*, traits::serde_adapters::*,
  values::result::*, qos::*, pubsub::Subscriber, topic::Topic, readcondition::*,
};

use crate::dds::datareader as datareader_with_key;
use crate::dds::datasample as datasample_with_key;
use crate::dds::no_key::datasample::*;

use crate::serialization;
use crate::messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier;

use std::sync::{Arc, RwLock};

// We should not expose the NoKeyWrapper type.
// TODO: Find a way how to remove the from read()'s return type and then hide this data type.
// NoKeyWrapper is defined separately for reading and writing so that
// they can require only either Serialize or Deserialize.
pub struct NoKeyWrapper<D> {
  pub d: D,
}

// implement Deref so that &NoKeyWrapper<D> is coercible to &D
impl<D> Deref for NoKeyWrapper<D> {
  type Target = D;
  fn deref(&self) -> &Self::Target {
    &self.d
  }
}

impl<D> Keyed for NoKeyWrapper<D> {
  type K = ();
  fn get_key(&self) -> () {
    ()
  }
}

impl<'de,D> Deserialize<'de> for NoKeyWrapper<D>
where  D: Deserialize<'de>,
{
  fn deserialize<R>(deserializer: R) -> std::result::Result<NoKeyWrapper<D>, R::Error>
  where
    R: Deserializer<'de>,
  {
    D::deserialize(deserializer).map(|d| NoKeyWrapper::<D> { d })
  }
}

struct SA_Wrapper<SA> {
  inner: SA,
}

impl<D:DeserializeOwned, SA:DeserializerAdapter<D> > DeserializerAdapter<NoKeyWrapper<D>> 
  for SA_Wrapper<SA> {

  fn supported_encodings() -> &'static [RepresentationIdentifier] {
    SA::supported_encodings()
  }
  fn from_bytes<'de>(input_bytes: &'de [u8], encoding: RepresentationIdentifier) 
    -> serialization::error::Result<NoKeyWrapper<D>> {
    SA::from_bytes(input_bytes,encoding).map(|d| NoKeyWrapper::<D> { d })
  }
}


impl<D> NoKeyWrapper<D> {}

// DataReader for NO_KEY data. Does not require "D: Keyed"
pub struct DataReader<'a, D,SA> {
  keyed_datareader: datareader_with_key::DataReader<'a, NoKeyWrapper<D>, SA_Wrapper<SA>>,
}

// TODO: rewrite DataSample so it can use current Keyed version (and send back datasamples instead of current data)
impl<'s, 'a, D, SA> DataReader<'a, D,SA>
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
  ) -> Self {
    DataReader {
      keyed_datareader: datareader_with_key::DataReader::<'a, NoKeyWrapper<D>,SA_Wrapper<SA> >::new(
        subscriber,
        my_id,
        topic,
        notification_receiver,
        dds_cache,
      ),
    }
  }

  pub fn read(
    &mut self,
    max_samples: usize,            // maximum number of DataSamples to return.
    read_condition: ReadCondition, // use e.g. ReadCondition::any() or ReadCondition::not_read()
  ) -> Result<Vec<&DataSample<NoKeyWrapper<D>>>> 
  {
    let kv: Result<Vec<&datasample_with_key::DataSample<NoKeyWrapper<D>>>> = self
      .keyed_datareader
      .read(max_samples, read_condition);
    #[allow(unused_variables)]
    kv.map(move |v| {
      v.iter()
        .map(
          move | &datasample_with_key::DataSample { sample_info, value }| DataSample {
            sample_info: sample_info.clone(),
            value: value.as_ref().expect("Received instance state change for no_key data. What to do?") 
              /*.as_ref()
              .expect("Received instance state change for no_key data. What to do?")
              .clone() */ ,
          },
        )
        .collect()
    })
  }

  // It does not make any sense to implement read_instance(), by definition of "no_key".
  /*
  pub fn read_instance(
    &self,
    take: Take,
    max_samples: usize,
    read_condition: ReadCondition,
    instance_key: Option<<D as Keyed>::K>,
    this_or_next: SelectByKey,
  ) -> Result<Vec<DataSample<D>>> {
    unimplemented!()
  }
  */

  /* TODO
  /// This is a simplified API for reading the next not_read sample
  /// If no new data is available, the return value is Ok(None).
   
  pub fn read_next_sample(&mut self) -> Result<Option<&DataSample<NoKeyWrapper<D>>>> {
    let mut ds = self.read(1, ReadCondition::not_read())?;
    Ok(ds.pop())
  } */
} // impl

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

impl<D,SA> HasQoSPolicy for DataReader<'_, D, SA> {
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
