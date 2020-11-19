use std::{fs::File, io};
use std::sync::{Arc, RwLock};
use std::marker::PhantomData;

use itertools::Itertools;
use io::Write;
use serde::de::DeserializeOwned;
use mio_extras::channel as mio_channel;
use log::{error, info, warn};
use mio::{Evented, Poll, PollOpt, Ready, Token};

use crate::{
  serialization::CDRDeserializerAdapter,
  discovery::discovery::DiscoveryCommand,
  structure::{
    entity::{Entity, EntityAttributes},
    guid::{GUID, EntityId},
    time::Timestamp,
    dds_cache::DDSCache,
    cache_change::{CacheChange, ChangeKind},
  },
};
use crate::dds::{
  traits::{key::*, TopicDescription},
  traits::serde_adapters::*,
  values::result::*,
  qos::*,
  with_key::datasample::*,
  sampleinfo::*,
  datasample_cache::DataSampleCache,
  pubsub::Subscriber,
  topic::Topic,
  readcondition::*,
};

use crate::messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier;

/// Parameter for reading [Readers](../struct.With_Key_DataReader.html) data with key or with next from current key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectByKey {
  This,
  Next,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderCommand {
  RESET_REQUESTED_DEADLINE_STATUS,
}

pub struct CurrentStatusChanges {
  pub livelinessLost: Option<LivelinessLostStatus>,
  pub offeredDeadlineMissed: Option<OfferedDeadlineMissedStatus>,
  pub offeredIncompatibleQos: Option<OfferedIncompatibleQosStatus>,
  pub requestedDeadlineMissed: Option<RequestedDeadlineMissedStatus>,
  pub requestedIncompatibleQos: Option<RequestedIncompatibleQosStatus>,
  pub publicationMatched: Option<PublicationMatchedStatus>,
  pub subscriptionMatched: Option<SubscriptionMatchedStatus>,
}

impl CurrentStatusChanges {
  pub fn new() -> CurrentStatusChanges {
    CurrentStatusChanges {
      livelinessLost: None,
      offeredDeadlineMissed: None,
      offeredIncompatibleQos: None,
      requestedDeadlineMissed: None,
      requestedIncompatibleQos: None,
      publicationMatched: None,
      subscriptionMatched: None,
    }
  }
}

pub struct DataReader<
  'a,
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D> = CDRDeserializerAdapter<D>,
> {
  my_subscriber: &'a Subscriber,
  my_topic: &'a Topic,
  qos_policy: QosPolicies,
  entity_attributes: EntityAttributes,
  pub(crate) notification_receiver: mio_channel::Receiver<()>,

  dds_cache: Arc<RwLock<DDSCache>>,

  datasample_cache: DataSampleCache<D>,
  latest_instant: Timestamp,
  deserializer_type: PhantomData<DA>, // This is to provide use for DA

  discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  status_receiver: mio_channel::Receiver<StatusChange>,
  current_status: CurrentStatusChanges,
  reader_command: mio_channel::SyncSender<ReaderCommand>,
}

impl<'a, D, DA> Drop for DataReader<'a, D, DA>
where
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  fn drop(&mut self) {
    match self
      .discovery_command
      .send(DiscoveryCommand::REMOVE_LOCAL_READER {
        guid: self.get_guid(),
      }) {
      Ok(_) => {}
      Err(e) => error!(
        "Failed to send REMOVE_LOCAL_READER DiscoveryCommand. {:?}",
        e
      ),
    }
  }
}

impl<'a, D: 'static, DA> DataReader<'a, D, DA>
where
  D: DeserializeOwned + Keyed,
  <D as Keyed>::K: Key,
  DA: DeserializerAdapter<D>,
{
  pub(crate) fn new(
    subscriber: &'a Subscriber,
    my_id: EntityId,
    topic: &'a Topic,
    // Each notification sent to this channel must be try_recv'd
    notification_receiver: mio_channel::Receiver<()>,
    dds_cache: Arc<RwLock<DDSCache>>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
    status_receiver: mio_channel::Receiver<StatusChange>,
    reader_command: mio_channel::SyncSender<ReaderCommand>,
  ) -> Result<Self> {
    let dp = match subscriber.get_participant() {
      Some(dp) => dp,
      None => {
        error!("Cannot create new DataReader, DomainParticipant doesn't exist.");
        return Err(Error::PreconditionNotMet);
      }
    };

    let entity_attributes = EntityAttributes::new(GUID::new_with_prefix_and_id(
      dp.get_guid_prefix().clone(),
      my_id,
    ));

    Ok(Self {
      my_subscriber: subscriber,
      my_topic: topic,
      qos_policy: topic.get_qos().clone(),
      entity_attributes,
      notification_receiver,
      dds_cache,
      datasample_cache: DataSampleCache::new(topic.get_qos().clone()),
      // The reader is created before the datareader, hence initializing the
      // latest_instant to now should be fine. There should be no smaller instants
      // added by the reader.
      latest_instant: Timestamp::now(),
      deserializer_type: PhantomData,
      discovery_command,
      status_receiver,
      current_status: CurrentStatusChanges::new(),
      reader_command,
    })
  }

  pub fn read(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<DataSample<&D>>> {
    self.fill_local_datasample_cache();

    let mut selected = self.datasample_cache.select_keys_for_access(read_condition);
    selected.truncate(max_samples);

    let result = self.datasample_cache.read_by_keys(&selected);
    // clearing receiver buffer
    while let Ok(_) = self.notification_receiver.try_recv() {}

    Ok(result)
  }

  pub fn take(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<DataSample<D>>> {
    self.fill_local_datasample_cache();

    let mut selected = self.datasample_cache.select_keys_for_access(read_condition);
    selected.truncate(max_samples);

    let result = self.datasample_cache.take_by_keys(&selected);

    // clearing receiver buffer
    while let Ok(_) = self.notification_receiver.try_recv() {}

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
  pub fn iterator(&mut self) -> Result<impl Iterator<Item = std::result::Result<&D, D::K>>> {
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
  ) -> Result<impl Iterator<Item = std::result::Result<&D, D::K>>> {
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
  pub fn into_iterator(&mut self) -> Result<impl Iterator<Item = std::result::Result<D, D::K>>> {
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
  ) -> Result<impl Iterator<Item = std::result::Result<D, D::K>>> {
    // TODO: We could come up with a more efficent implementation than wrapping a read call
    Ok(
      self
        .take(std::usize::MAX, read_condition)?
        .into_iter()
        .map(|ds| ds.value),
    )
  }

  // Gets all unseen cache_changes from the TopicCache. Deserializes
  // the serialized payload and stores the DataSamples (the actual data and the
  // samplestate) to local container, datasample_cache.
  fn fill_local_datasample_cache(&mut self) {
    let dds_cache = match self.dds_cache.read() {
      Ok(rwlock) => rwlock,
      // TODO: Should we panic here? Are we allowed to continue with poisoned DDSCache?
      Err(e) => panic!(
        "The DDSCache of domain participant is poisoned. Error: {}",
        e
      ),
    };

    let cache_changes = dds_cache.from_topic_get_changes_in_range(
      &self.my_topic.get_name().to_string(),
      &self.latest_instant,
      &Timestamp::now(),
    );

    let cache_changes: Vec<(&Timestamp, &CacheChange)> = cache_changes
      .into_iter()
      .sorted_by(|(a, _), (b, _)| Ord::cmp(a, b))
      .filter(|(_, cc)| cc.writer_guid.guidPrefix != self.get_guid_prefix())
      .collect();

    match cache_changes.last() {
      Some((last_instant, _)) => self.latest_instant = **last_instant,
      // No new changes available
      None => return,
    };

    for (
      instant,
      CacheChange {
        kind,
        writer_guid,
        sequence_number: _,
        data_value: payload_opt,
        key: key_hash,
      },
    ) in cache_changes
    {
      match kind {
        ChangeKind::NOT_ALIVE_UNREGISTERED => (), // presumably causes no local cache update?

        ChangeKind::NOT_ALIVE_DISPOSED => {
          /* TODO: Instance to be disposed could be specified by serialized payload also, not only key_hash? */
          match self.datasample_cache.get_key_by_hash(*key_hash) {
            Some(key) => self
              .datasample_cache
              .add_sample(Err(key), *writer_guid, *instant, None),
            /* TODO: How to get source timestamps other then None ?? */
            None => warn!("Tried to dispose with unkonwn key hash: {:x?}", key_hash),
          }
        }
        ChangeKind::ALIVE => {
          match payload_opt {
            Some(serialized_payload) => {
              // what is our data serialization format (representation identifier) ?
              let rep_id = match RepresentationIdentifier::try_from_u16(
                serialized_payload.representation_identifier,
              ) {
                Ok(r) => r,
                // cannot use .or_else() because need to "continue" the for-loop
                Err(other_rep_id) => {
                  if let Some(ri) = DA::supported_encodings()
                    .iter()
                    .find(|r| **r as u16 == other_rep_id)
                  {
                    *ri // no worries, our DeserializerAdapter recognizes this representation
                  } else {
                    warn!("Datareader: Unknown representation id {:?}.", other_rep_id);
                    continue; // skip this sample, as we cannot decode it
                  }
                }
              };

              // deserialize
              let payload = match DA::from_bytes(&serialized_payload.value, rep_id) {
                Ok(p) => p,
                // cannot use .or_else() because need to "continue" the for-loop
                Err(e) => {
                  error!("Failed to deserialize bytes \n{}", e);
                  // TODO: Wrap this in a debug conditional. We cannot go writing
                  // to the file system unless requested by user!
                  File::create("error_bin.bin")
                    .unwrap()
                    .write_all(&serialized_payload.value)
                    .unwrap();
                  continue;
                }
              };
              // insert to local cache
              self
                .datasample_cache
                .add_sample(Ok(payload), *writer_guid, *instant, None)
              /* TODO: How to get source timestamps other then None ?? */
            }
            None => warn!("Got CacheChange kind=ALIVE , but no serialized payload!"),
          }
        }
      }
    }
  }

  fn infer_key(
    &self,
    instance_key: Option<<D as Keyed>::K>,
    this_or_next: SelectByKey,
  ) -> Option<<D as Keyed>::K> {
    match instance_key {
      Some(k) => match this_or_next {
        SelectByKey::This => Some(k),
        SelectByKey::Next => self.datasample_cache.get_next_key(&k),
      },
      None => self
        .datasample_cache
        .instance_map
        .keys()
        .next()
        .map(|k| k.clone()),
    }
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
  ) -> Result<Vec<DataSample<&D>>> {
    self.fill_local_datasample_cache();

    let key = match self.infer_key(instance_key, this_or_next) {
      Some(k) => k,
      None => return Ok(Vec::new()),
    };

    let mut selected = self
      .datasample_cache
      .select_instance_keys_for_access(key, read_condition);
    selected.truncate(max_samples);

    let result = self.datasample_cache.read_by_keys(&selected);

    // clearing receiver buffer
    while let Ok(_) = self.notification_receiver.try_recv() {}

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
    self.fill_local_datasample_cache();

    let key = match self.infer_key(instance_key, this_or_next) {
      Some(k) => k,
      None => return Ok(Vec::new()),
    };

    let mut selected = self
      .datasample_cache
      .select_instance_keys_for_access(key, read_condition);
    selected.truncate(max_samples);

    let result = self.datasample_cache.take_by_keys(&selected);

    // clearing receiver buffer
    while let Ok(_) = self.notification_receiver.try_recv() {}

    Ok(result)
  }

  // status queries

  fn reset_local_requested_deadline_status_change(&mut self) {
    info!(
      "reset_local_requested_deadline_status_change current {:?}",
      self.current_status.requestedDeadlineMissed
    );
    match self.current_status.requestedDeadlineMissed {
      Some(_s) => {
        self
          .current_status
          .requestedDeadlineMissed
          .as_mut()
          .unwrap()
          .reset_change();
      }
      None => {}
    }
  }

  fn fetch_readers_current_status(&mut self) -> Result<()> {
    //self.TEST_FUNCTION_get_requested_deadline_missed_status();
    let mut received_requested_deadline_status_change: bool = false;
    loop {
      let received_status = self.status_receiver.try_recv();
      match received_status {
        Ok(s) => match s {
          StatusChange::LivelinessLostStatus(status) => {
            self.current_status.livelinessLost = Some(status);
          }
          StatusChange::OfferedDeadlineMissedStatus(status) => {
            self.current_status.offeredDeadlineMissed = Some(status);
          }
          StatusChange::OfferedIncompatibleQosStatus(status) => {
            self.current_status.offeredIncompatibleQos = Some(status);
          }
          StatusChange::RequestedDeadlineMissedStatus(status) => {
            self.current_status.requestedDeadlineMissed = Some(status);
            received_requested_deadline_status_change = true;
          }
          StatusChange::RequestedIncompatibleQosStatus(status) => {
            self.current_status.requestedIncompatibleQos = Some(status);
          }
          StatusChange::PublicationMatchedStatus(status) => {
            self.current_status.publicationMatched = Some(status);
          }
          StatusChange::SubscriptionMatchedStatus(status) => {
            self.current_status.subscriptionMatched = Some(status);
          }
        },
        Err(e) => {
          match e {
            std::sync::mpsc::TryRecvError::Empty => {
              break;
            }
            std::sync::mpsc::TryRecvError::Disconnected => {
              error!(
                " Reader disconnected, could not fetch status change: {:?}",
                e
              );
              // return disconnect status!!!
              return Err(Error::OutOfResources);
            }
          }
        }
      }
    }
    // after looping set command to reset requested deadeline status if needed. Also reset local status.
    match received_requested_deadline_status_change {
      true => {
        self
          .current_status
          .requestedDeadlineMissed
          .unwrap()
          .reset_change();
        match self
          .reader_command
          .try_send(ReaderCommand::RESET_REQUESTED_DEADLINE_STATUS)
        {
          Ok(()) => {
            return Ok(());
          }
          Err(e) => {
            error!("Unable to send RESET_REQUESTED_DEADLINE_STATUS: {:?}", e);
            return Err(Error::OutOfResources);
          }
        }
      }
      false => {
        return Ok(());
      }
    }
  }
  /*
  pub fn register_status_change_notificator(&self, poll: & Poll, token: Token, interest: Ready, opts: PollOpt) -> Result<()>{
    todo!();
    // let res = poll.register(&self.status_receiver, token, interest, opts);

    //self.TEST_FUNCTION_get_requested_deadline_missed_status();
    //info!("register datareader status reciever to poll with token {:?}", token );
    //return res;
  }
  */

  // Helper functions

  fn matches_conditions(rcondition: &ReadCondition, dsample: &DataSample<D>) -> bool {
    if !rcondition
      .sample_state_mask()
      .contains(dsample.sample_info().sample_state)
    {
      return false;
    }
    if !rcondition
      .view_state_mask()
      .contains(dsample.sample_info().view_state)
    {
      return false;
    }
    if !rcondition
      .instance_state_mask()
      .contains(dsample.sample_info().instance_state)
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

  #[cfg(test)]
  pub fn TEST_FUNCTION_set_status_change_receiver(
    &mut self,
    receiver: mio_channel::Receiver<StatusChange>,
  ) {
    self.status_receiver = receiver;
  }

  #[cfg(test)]
  pub fn TEST_FUNCTION_get_requested_deadline_missed_status(
    &mut self,
  ) -> Result<Option<RequestedDeadlineMissedStatus>> {
    self.get_requested_deadline_missed_status()
  }

  #[cfg(test)]
  pub fn TEST_FUNCTION_set_reader_commander(
    &mut self,
    sender: mio_channel::SyncSender<ReaderCommand>,
  ) {
    self.reader_command = sender;
  }

  pub fn get_requested_deadline_missed_status(
    &mut self,
  ) -> Result<Option<RequestedDeadlineMissedStatus>> {
    self.fetch_readers_current_status()?;
    let value_before_reset = self.current_status.requestedDeadlineMissed.clone();
    self.reset_local_requested_deadline_status_change();
    return Ok(value_before_reset);
  }
} // impl

/*
impl<'a, D: 'static, SA> IDataReader<D, SA> for DataReader<'a, D, SA>
where
  D: DeserializeOwned + Keyed,
  <D as Keyed>::K: Key,
  SA: DeserializerAdapter<D>,
{
  fn read(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<&dyn IDataSample<D>>> {
    let samples = self.read_as_obj(max_samples, read_condition);
    match samples {
      Ok(d) => Ok(d.into_iter().map(|p| p.as_idata_sample()).collect()),
      Err(e) => Err(e),
    }
  }

  fn take(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<Box<dyn IDataSample<D>>>> {
    let samples = self.take_as_obj(max_samples, read_condition);
    match samples {
      Ok(d) => Ok(d.into_iter().map(|p| p.into_idata_sample()).collect()),
      Err(e) => Err(e),
    }
  }

  fn read_next_sample(&mut self) -> Result<Option<&dyn IDataSample<D>>> {
    let mut ds =
      <DataReader<D, SA> as IDataReader<D, SA>>::read(self, 1, ReadCondition::not_read())?;
    let val = match ds.pop() {
      Some(v) => Some(v.as_idata_sample()),
      None => None,
    };
    Ok(val)
  }

  fn take_next_sample(&mut self) -> Result<Option<Box<dyn IDataSample<D>>>> {
    let ds = self.take_next_sample()?;
    Ok(ds.into_iter().map(|p| p.into_idata_sample()).find(|_| true))
  }

  fn get_requested_deadline_missed_status(&mut self) -> Result<Option<RequestedDeadlineMissedStatus>> {
    self.fetch_readers_current_status()?;
    let value_before_reset = self.current_status.requestedDeadlineMissed.clone();
    self.reset_local_requested_deadline_status_change();
    return Ok(value_before_reset);

  }
}
*/

// This is  not part of DDS spec. We implement mio Eventd so that the application can asynchronously
// poll DataReader(s).
impl<'a, D, DA> Evented for DataReader<'a, D, DA>
where
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D>,
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

impl<D, DA> HasQoSPolicy for DataReader<'_, D, DA>
where
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D>,
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

impl<'a, D, DA> Entity for DataReader<'a, D, DA>
where
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dds::{participant::DomainParticipant, topic::TopicKind};
  use crate::test::random_data::*;
  use crate::dds::traits::key::Keyed;
  use mio_extras::channel as mio_channel;
  use log::info;
  use crate::dds::reader::Reader;
  use crate::messages::submessages::data::Data;
  use crate::dds::message_receiver::*;
  use crate::structure::guid::GuidPrefix;
  use crate::structure::sequence_number::SequenceNumber;
  use crate::serialization::{cdr_deserializer::CDRDeserializerAdapter, cdr_serializer::to_bytes};
  use byteorder::LittleEndian;
  use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
  use std::{
    thread,
    time::{self},
  };
  use mio::{Events};
  #[test]
  fn dr_get_samples_from_ddschache() {
    let dp = DomainParticipant::new(0);
    let mut qos = QosPolicies::qos_none();
    qos.history = Some(policy::History::KeepAll);

    let sub = dp.create_subscriber(&qos).unwrap();
    let topic = dp
      .create_topic("dr", "drtest?", &qos, TopicKind::WithKey)
      .unwrap();

    let (send, _rec) = mio_channel::sync_channel::<()>(10);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<StatusChange>(100);
    let (_reader_commander, reader_command_receiver) =
      mio_extras::channel::sync_channel::<ReaderCommand>(100);

    let reader_id = EntityId::default();
    let datareader_id = EntityId::default();
    let reader_guid = GUID::new_with_prefix_and_id(dp.get_guid_prefix(), reader_id);

    let mut new_reader = Reader::new(
      reader_guid,
      send,
      status_sender,
      dp.get_dds_cache(),
      topic.get_name().to_string(),
      reader_command_receiver,
    );

    let mut matching_datareader = sub
      .create_datareader::<RandomData, CDRDeserializerAdapter<RandomData>>(
        &topic,
        Some(datareader_id),
        None,
      )
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

    new_reader.matched_writer_add(
      writer_guid.clone(),
      EntityId::ENTITYID_UNKNOWN,
      mr_state.unicast_reply_locator_list.clone(),
      mr_state.multicast_reply_locator_list.clone(),
    );

    let mut data = Data::default();
    data.reader_id = EntityId::createCustomEntityID([1, 2, 3], 111);
    data.writer_id = writer_guid.entityId;
    data.writer_sn = SequenceNumber::from(0);

    data.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, LittleEndian>(&random_data).unwrap(),
    });
    new_reader.handle_data_msg(data, mr_state.clone());

    matching_datareader.fill_local_datasample_cache();
    let deserialized_random_data = matching_datareader.read(1, ReadCondition::any()).unwrap()[0]
      .value()
      .unwrap()
      .clone();

    assert_eq!(deserialized_random_data, random_data);

    // Test getting of next samples.
    let random_data2 = RandomData {
      a: 1,
      b: "somedata number 2".to_string(),
    };
    let mut data2 = Data::default();
    data2.reader_id = EntityId::createCustomEntityID([1, 2, 3], 111);
    data2.writer_id = writer_guid.entityId;
    data2.writer_sn = SequenceNumber::from(1);

    data2.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, LittleEndian>(&random_data2).unwrap(),
    });

    let random_data3 = RandomData {
      a: 1,
      b: "third somedata".to_string(),
    };
    let mut data3 = Data::default();
    data3.reader_id = EntityId::createCustomEntityID([1, 2, 3], 111);
    data3.writer_id = writer_guid.entityId;
    data3.writer_sn = SequenceNumber::from(2);

    data3.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, LittleEndian>(&random_data3).unwrap(),
    });

    new_reader.handle_data_msg(data2, mr_state.clone());
    new_reader.handle_data_msg(data3, mr_state);

    matching_datareader.fill_local_datasample_cache();
    let random_data_vec = matching_datareader
      .read_instance(100, ReadCondition::any(), Some(data_key), SelectByKey::This)
      .unwrap();
    assert_eq!(random_data_vec.len(), 3);
  }

  #[test]
  fn dr_read_and_take() {
    let dp = DomainParticipant::new(0);

    let mut qos = QosPolicies::qos_none();
    qos.history = Some(policy::History::KeepAll); // Just for testing

    let sub = dp.create_subscriber(&qos).unwrap();
    let topic = dp
      .create_topic("dr read", "read fn test?", &qos, TopicKind::WithKey)
      .unwrap();

    let (send, _rec) = mio_channel::sync_channel::<()>(10);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<StatusChange>(100);
    let (_reader_commander, reader_command_receiver) =
      mio_extras::channel::sync_channel::<ReaderCommand>(100);

    let default_id = EntityId::default();
    let reader_guid = GUID::new_with_prefix_and_id(dp.get_guid_prefix(), default_id);

    let mut reader = Reader::new(
      reader_guid,
      send,
      status_sender,
      dp.get_dds_cache(),
      topic.get_name().to_string(),
      reader_command_receiver,
    );

    let mut datareader = sub
      .create_datareader::<RandomData, CDRDeserializerAdapter<RandomData>>(
        &topic,
        Some(default_id),
        None,
      )
      .unwrap();

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };
    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;
    reader.matched_writer_add(
      writer_guid.clone(),
      EntityId::ENTITYID_UNKNOWN,
      mr_state.unicast_reply_locator_list.clone(),
      mr_state.multicast_reply_locator_list.clone(),
    );

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
    data_msg.reader_id = reader.get_entity_id();
    data_msg.writer_id = writer_guid.entityId;
    data_msg.writer_sn = SequenceNumber::from(0);

    data_msg.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, LittleEndian>(&test_data).unwrap(),
    });

    let mut data_msg2 = Data::default();
    data_msg2.reader_id = reader.get_entity_id();
    data_msg2.writer_id = writer_guid.entityId;
    data_msg2.writer_sn = SequenceNumber::from(1);

    data_msg2.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, LittleEndian>(&test_data2).unwrap(),
    });
    reader.handle_data_msg(data_msg, mr_state.clone());
    reader.handle_data_msg(data_msg2, mr_state.clone());

    // Read the same sample two times.
    {
      let result_vec = datareader.read(100, ReadCondition::any()).unwrap();
      let d = result_vec[0].value().unwrap();
      assert_eq!(&test_data, d);
    }
    {
      let result_vec2 = datareader.read(100, ReadCondition::any()).unwrap();
      let d2 = result_vec2[1].value().unwrap();
      assert_eq!(&test_data2, d2);
    }
    {
      let result_vec3 = datareader.read(100, ReadCondition::any()).unwrap();
      let d3 = result_vec3[0].value().unwrap();
      assert_eq!(&test_data, d3);
    }

    // Take
    let mut result_vec = datareader.take(100, ReadCondition::any()).unwrap();
    let result_vec2 = datareader.take(100, ReadCondition::any());

    let d2 = result_vec.pop().unwrap();
    let d2 = d2.value().as_ref().unwrap().clone();
    let d1 = result_vec.pop().unwrap();
    let d1 = d1.value().as_ref().unwrap().clone();
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
    data_msg.reader_id = reader.get_entity_id();
    data_msg.writer_id = writer_guid.entityId;
    data_msg.writer_sn = SequenceNumber::from(2);

    data_msg.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, LittleEndian>(&data_key1).unwrap(),
    });
    let mut data_msg2 = Data::default();
    data_msg2.reader_id = reader.get_entity_id();
    data_msg2.writer_id = writer_guid.entityId;
    data_msg2.writer_sn = SequenceNumber::from(3);

    data_msg2.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, LittleEndian>(&data_key2_1).unwrap(),
    });
    let mut data_msg3 = Data::default();
    data_msg3.reader_id = reader.get_entity_id();
    data_msg3.writer_id = writer_guid.entityId;
    data_msg3.writer_sn = SequenceNumber::from(4);

    data_msg3.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, LittleEndian>(&data_key2_2).unwrap(),
    });
    let mut data_msg4 = Data::default();
    data_msg4.reader_id = reader.get_entity_id();
    data_msg4.writer_id = writer_guid.entityId;
    data_msg4.writer_sn = SequenceNumber::from(5);

    data_msg4.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, LittleEndian>(&data_key2_3).unwrap(),
    });
    reader.handle_data_msg(data_msg, mr_state.clone());
    reader.handle_data_msg(data_msg2, mr_state.clone());
    reader.handle_data_msg(data_msg3, mr_state.clone());
    reader.handle_data_msg(data_msg4, mr_state.clone());

    info!("calling read with key 1 and this");
    let results =
      datareader.read_instance(100, ReadCondition::any(), Some(key1), SelectByKey::This);
    assert_eq!(data_key1, results.unwrap()[0].value().unwrap().clone());

    info!("calling read with None and this");
    // Takes the samllest key, 1 in this case.
    let results = datareader.read_instance(100, ReadCondition::any(), None, SelectByKey::This);
    assert_eq!(data_key1, results.unwrap()[0].value().unwrap().clone());

    info!("calling read with key 1 and next");
    let results =
      datareader.read_instance(100, ReadCondition::any(), Some(key1), SelectByKey::Next);
    assert_eq!(results.as_ref().unwrap().len(), 3);
    assert_eq!(data_key2_2, results.unwrap()[1].value().unwrap().clone());

    info!("calling take with key 2 and this");
    let results =
      datareader.take_instance(100, ReadCondition::any(), Some(key2), SelectByKey::This);
    assert_eq!(results.as_ref().unwrap().len(), 3);
    let mut vec = results.unwrap();
    let d3 = vec.pop().unwrap();
    let d3 = d3.into_value().unwrap();
    let d2 = vec.pop().unwrap();
    let d2 = d2.into_value().unwrap();
    let d1 = vec.pop().unwrap();
    let d1 = d1.into_value().unwrap();
    assert_eq!(data_key2_3, d3);
    assert_eq!(data_key2_2, d2);
    assert_eq!(data_key2_1, d1);

    info!("calling take with key 2 and this");
    let results =
      datareader.take_instance(100, ReadCondition::any(), Some(key2), SelectByKey::This);
    assert!(results.is_ok());
    assert!(results.unwrap().is_empty());
  }

  #[test]
  fn dr_wake_up() {
    let dp = DomainParticipant::new(0);

    let mut qos = QosPolicies::qos_none();
    qos.history = Some(policy::History::KeepAll); // Just for testing

    let sub = dp.create_subscriber(&qos).unwrap();
    let topic = dp
      .create_topic("wakeup", "Wake up!", &qos, TopicKind::WithKey)
      .unwrap();

    let (send, rec) = mio_channel::sync_channel::<()>(10);
    let (status_sender, _status_reciever) = mio_extras::channel::sync_channel::<StatusChange>(100);
    let (_reader_commander, reader_command_receiver) =
      mio_extras::channel::sync_channel::<ReaderCommand>(100);

    let default_id = EntityId::default();
    let reader_guid = GUID::new_with_prefix_and_id(dp.get_guid_prefix(), default_id);

    let mut reader = Reader::new(
      reader_guid,
      send,
      status_sender,
      dp.get_dds_cache(),
      topic.get_name().to_string(),
      reader_command_receiver,
    );

    let mut datareader = sub
      .create_datareader::<RandomData, CDRDeserializerAdapter<RandomData>>(
        &topic,
        Some(default_id),
        None,
      )
      .unwrap();
    datareader.notification_receiver = rec;

    let writer_guid = GUID {
      guidPrefix: GuidPrefix::new(vec![1; 12]),
      entityId: EntityId::createCustomEntityID([1; 3], 1),
    };
    let mut mr_state = MessageReceiverState::default();
    mr_state.source_guid_prefix = writer_guid.guidPrefix;
    reader.matched_writer_add(
      writer_guid.clone(),
      EntityId::ENTITYID_UNKNOWN,
      mr_state.unicast_reply_locator_list.clone(),
      mr_state.multicast_reply_locator_list.clone(),
    );

    let test_data1 = RandomData {
      a: 1,
      b: "Testing 1".to_string(),
    };

    let test_data2 = RandomData {
      a: 2,
      b: "Testing 2".to_string(),
    };

    let test_data3 = RandomData {
      a: 2,
      b: "Testing 3".to_string(),
    };

    let mut data_msg = Data::default();
    data_msg.reader_id = reader.get_entity_id();
    data_msg.writer_id = writer_guid.entityId;
    data_msg.writer_sn = SequenceNumber::from(0);

    data_msg.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, byteorder::LittleEndian>(&test_data1).unwrap(),
    });

    let mut data_msg2 = Data::default();
    data_msg2.reader_id = reader.get_entity_id();
    data_msg2.writer_id = writer_guid.entityId;
    data_msg2.writer_sn = SequenceNumber::from(1);

    data_msg2.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, byteorder::LittleEndian>(&test_data2).unwrap(),
    });

    let mut data_msg3 = Data::default();
    data_msg3.reader_id = reader.get_entity_id();
    data_msg3.writer_id = writer_guid.entityId;
    data_msg3.writer_sn = SequenceNumber::from(2);

    data_msg3.serialized_payload = Some(SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE as u16,
      representation_options: [0, 0],
      value: to_bytes::<RandomData, byteorder::LittleEndian>(&test_data3).unwrap(),
    });

    let handle = std::thread::spawn(move || {
      reader.handle_data_msg(data_msg, mr_state.clone());
      thread::sleep(time::Duration::from_millis(2500));
      info!("I'll send the second now..");
      reader.handle_data_msg(data_msg2, mr_state.clone());
      thread::sleep(time::Duration::from_millis(2500));
      info!("I'll send the third now..");
      reader.handle_data_msg(data_msg3, mr_state.clone());
    });

    let poll = Poll::new().unwrap();
    poll
      .register(&datareader, Token(100), Ready::readable(), PollOpt::edge())
      .unwrap();

    let mut count_to_stop = 0;
    'l: loop {
      let mut events = Events::with_capacity(1024);
      info!("Going to poll");
      poll.poll(&mut events, None).unwrap();

      for event in events.into_iter() {
        info!("Handling events");
        if event.token() == Token(100) {
          let data = datareader.take(100, ReadCondition::any());
          let len = data.as_ref().unwrap().len();
          info!("There were {} samples available.", len);
          info!("Their strings:");
          for d in data.unwrap().into_iter() {
            // Remove one notification for each data
            info!("{}", d.value().as_ref().unwrap().b);
          }
          count_to_stop += len;
        }
        if count_to_stop >= 3 {
          info!("I'll stop now with count {}", count_to_stop);
          break 'l;
        }
      } // for
    } // loop

    handle.join().unwrap();
    assert_eq!(count_to_stop, 3);
  }
}
