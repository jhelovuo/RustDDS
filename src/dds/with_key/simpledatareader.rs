use std::{
  cmp::max,
  io,
  marker::PhantomData,
  sync::{Arc, RwLock, Mutex,},
  collections::{BTreeMap},
  task::Waker,
};

use serde::de::DeserializeOwned;
use mio_extras::channel as mio_channel;
#[allow(unused_imports)]
use log::{debug, error, info, warn, trace};

use mio_06::Evented;
use mio_06;

use mio_08;

use crate::{
  dds::{
    pubsub::Subscriber,
    qos::*,
    statusevents::*,
    topic::{Topic, TopicDescription,},
    traits::{key::*, serde_adapters::with_key::*, },
    values::result::*,
    ddsdata::*,
  },
  discovery::{ discovery::DiscoveryCommand},
  log_and_err_precondition_not_met,
  serialization::CDRDeserializerAdapter,
  structure::{
    dds_cache::DDSCache,
    cache_change::{CacheChange, DeserializedCacheChange},
    entity::RTPSEntity,
    guid::{EntityId, GUID},
    time::Timestamp,
    sequence_number::SequenceNumber,
  },
  mio_source::PollEventSource,
};

#[derive(Clone, Debug,)]
pub enum DataReaderWaker {
  FutureWaker(Waker),
  NoWaker,
}

impl DataReaderWaker {
  pub fn wake(&mut self) {
    use DataReaderWaker::*;
    match self {
      NoWaker => (),
      // Mio08Token{ token, notifier } => 
      //   notifier.notify(token_to_notification_id(token.clone()))
      //     .unwrap_or_else(|e| error!("Reader cannot notify Datareader: {:?}",e)),
      FutureWaker(fut_waker) => {
        fut_waker.wake_by_ref();
        *self = NoWaker;
      }
    }
  }
}

#[derive(Clone, Debug,)]
pub(crate) enum ReaderCommand {
  #[allow(dead_code)] // TODO: Implement this (resetting) feature
  ResetRequestedDeadlineStatus,
}

struct ReadPointers {
  latest_instant: Timestamp, // This is used as a read pointer from dds_cache for BEST_EFFORT reading
  last_read_sn : BTreeMap<GUID,SequenceNumber>, // collection of read pointers for RELIABLE reading
}

impl ReadPointers {
  fn new() -> Self {
    ReadPointers {
      latest_instant: Timestamp::ZERO,
      last_read_sn: BTreeMap::new(),
    }
  }
}

/// SimpleDataReaders can only do "take" semantics and does not have 
/// any deduplication or other DataSampleCache functionality.
pub struct SimpleDataReader<
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D> = CDRDeserializerAdapter<D>,
> 
{
  #[allow(dead_code)] // TODO: This is currently unused, because we do not implement
  // any subscriber-wide QoS policies, such as ordered or coherent access.
  // Remove this attribute when/if such things are implemented.
  my_subscriber: Subscriber,

  my_topic: Topic,
  qos_policy: QosPolicies,
  my_guid: GUID,
  pub(crate) notification_receiver: mio_channel::Receiver<()>,

  dds_cache: Arc<RwLock<DDSCache>>, // global cache

  read_pointers: ReadPointers,

  /// hask_to_key_map is used for decoding received key hashes back to original key values.
  /// This is needed when we receive a dispose message via hash only.
  hash_to_key_map: BTreeMap<KeyHash, D::K>, // TODO: garbage collect this somehow

  deserializer_type: PhantomData<DA>, // This is to provide use for DA

  discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  status_receiver: StatusReceiver<DataReaderStatus>,

  #[allow(dead_code)] // TODO: This is currently unused, because we do not implement
  // resetting deadline missed status. Remove attribute when it is supported.
  reader_command: mio_channel::SyncSender<ReaderCommand>,
  data_reader_waker: Arc<Mutex<DataReaderWaker>>,

  event_source: PollEventSource,
}



impl<D, DA> Drop for SimpleDataReader<D, DA>
where
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  fn drop(&mut self) {
    // Tell dp_event_loop
    self.my_subscriber.remove_reader(self.my_guid);

    // Tell discovery
    match self
      .discovery_command
      .send(DiscoveryCommand::RemoveLocalReader { guid: self.my_guid })
    {
      Ok(_) => {}
      Err(mio_channel::SendError::Disconnected(_)) => {
        debug!("Failed to send DiscoveryCommand::RemoveLocalReader . Maybe shutting down?");
      }
      Err(e) => error!(
        "Failed to send DiscoveryCommand::RemoveLocalReader. {:?}",
        e
      ),
    }
  }
}

impl<D: 'static, DA> SimpleDataReader<D, DA>
where
  D: DeserializeOwned + Keyed,
  <D as Keyed>::K: Key,
  DA: DeserializerAdapter<D>,
{
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new(
    subscriber: Subscriber,
    my_id: EntityId,
    topic: Topic,
    qos_policy: QosPolicies,
    // Each notification sent to this channel must be try_recv'd
    notification_receiver: mio_channel::Receiver<()>,
    dds_cache: Arc<RwLock<DDSCache>>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
    status_channel_rec: StatusChannelReceiver<DataReaderStatus>,
    reader_command: mio_channel::SyncSender<ReaderCommand>,
    data_reader_waker: Arc<Mutex<DataReaderWaker>>,
    event_source: PollEventSource,
  ) -> Result<Self> {
    let dp = match subscriber.participant() {
      Some(dp) => dp,
      None => {
        return log_and_err_precondition_not_met!(
          "Cannot create new DataReader, DomainParticipant doesn't exist."
        )
      }
    };

    let my_guid = GUID::new_with_prefix_and_id(dp.guid_prefix(), my_id);

    Ok(Self {
      my_subscriber: subscriber,
      qos_policy,
      my_guid,
      notification_receiver,
      dds_cache,
      read_pointers: ReadPointers::new(),
      hash_to_key_map: BTreeMap::new(),
      my_topic: topic,
      deserializer_type: PhantomData,
      discovery_command,
      status_receiver: StatusReceiver::new(status_channel_rec),
      reader_command,
      data_reader_waker,
      event_source,
    })
  }
  pub fn set_waker(&self, w: DataReaderWaker) {
    *self.data_reader_waker.lock().unwrap() = w;
  }

  pub(crate) fn drain_read_notifications(&self) {
    while self.notification_receiver.try_recv().is_ok() {}
    self.event_source.drain();
  }

  // get an iterator into DDS Cache, so this is in constant space, not a large data structure
  fn try_take_undecoded<'a>(is_reliable: bool, dds_cache: &'a DDSCache, 
      topic_name:&str, read_pointers: &'a ReadPointers ) 
    -> Box<dyn Iterator<Item = (Timestamp, &'a CacheChange)> + 'a>
  {
    if is_reliable {
      dds_cache.get_changes_in_range_reliable(
        topic_name,
        &read_pointers.last_read_sn,
      )
    } else {
      dds_cache.get_changes_in_range_best_effort(
        topic_name,
        read_pointers.latest_instant,
        Timestamp::now(),
      )
    }
  }


  fn update_hash_to_key_map(
      hash_to_key_map: &mut BTreeMap<KeyHash, D::K>, 
      deserialized: &std::result::Result<D, D::K>) 
  {
    let instance_key = match deserialized {
      Ok(d) => d.key(),
      Err(k) => k.clone(),
    };
    hash_to_key_map
      .insert(instance_key.hash_key(), instance_key.clone());
  }

  fn deserialize(timestamp: Timestamp, cc: &CacheChange, hash_to_key_map: &mut BTreeMap<KeyHash, D::K>) -> 
    std::result::Result<DeserializedCacheChange<D>,String>
  {
    match cc.data_value {
      DDSData::Data { ref serialized_payload } => {
        // what is our data serialization format (representation identifier) ?
        if let Some(recognized_rep_id) = DA::supported_encodings()
          .iter()
          .find(|r| **r == serialized_payload.representation_identifier)
        {
          match DA::from_bytes(&serialized_payload.value, *recognized_rep_id) {
            // Data update, decoded ok
            Ok(payload) => {
              let p = Ok(payload);
              Self::update_hash_to_key_map(hash_to_key_map,  &p);
              Ok(DeserializedCacheChange::new(timestamp,&cc,p))
            }
            Err(e) => Err(format!("Failed to deserialize sample bytes: {e}, ")),
          }
        } else {
          Err( format!("Unknown representation id {:?}.", 
                serialized_payload.representation_identifier))
        }
      }

      DDSData::DisposeByKey {
        key: ref serialized_key,
        ..
      } => {
        match DA::key_from_bytes(
          &serialized_key.value,
          serialized_key.representation_identifier,
        ) {
          Ok(key) => {
            let k = Err(key);
            Self::update_hash_to_key_map(hash_to_key_map,  &k);
            Ok(DeserializedCacheChange::new(timestamp,&cc,k))
          }
          Err(e) => Err( format!("Failed to deserialize key {}", e)),
        }
      }

      DDSData::DisposeByKeyHash { key_hash, .. } => {
        // The cache should know hash -> key mapping even if the sample
        // has been disposed or .take()n
        if let Some(key) = hash_to_key_map.get(&key_hash) {
          Ok(DeserializedCacheChange::new(timestamp, &cc,Err(key.clone())))
        } else {
          Err( format!("Tried to dispose with unknown key hash: {:x?}", key_hash) )
        }
      } 
    } // match    
  }

  pub fn try_take_one(&mut self) -> Result<Option<DeserializedCacheChange<D>>> {
    let is_reliable = matches!(
      self.qos_policy.reliability(),
      Some(policy::Reliability::Reliable { .. })
    );
    let dds_cache = match self.dds_cache.read() {
      Ok(rwlock) => rwlock,
      // TODO: Should we panic here? Are we allowed to continue with poisoned DDSCache?
      Err(e) => panic!(
        "The DDSCache of domain participant is poisoned. Error: {}",
        e
      ),
    };
    let (timestamp, cc) = 
      match Self::try_take_undecoded(is_reliable, &dds_cache, &self.my_topic.name(), &self.read_pointers)
              .next() {
        None => return Ok(None),
        Some((ts,cc)) => (ts,cc),
      };

    match Self::deserialize(timestamp, cc, &mut self.hash_to_key_map ) {
      Ok(dcc) => {
        self.read_pointers.latest_instant = max(self.read_pointers.latest_instant , timestamp);
        self.read_pointers.last_read_sn.insert(dcc.writer_guid, dcc.sequence_number);
        Ok(Some(dcc))
      } 
      Err(string) => {        
        Error::serialization_error(
          format!("{} Topic = {}, Type = {:?}",string,self.my_topic.name(), self.my_topic.get_type()))
      }
    }
  }

  pub fn qos(&self) -> &QosPolicies {
    &self.qos_policy
  }

  pub fn guid(&self) -> GUID {
    self.my_guid
  }
}


// This is  not part of DDS spec. We implement mio Eventd so that the
// application can asynchronously poll DataReader(s).
impl<D, DA> Evented for SimpleDataReader<D, DA>
where
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  // We just delegate all the operations to notification_receiver, since it
  // already implements Evented
  fn register(&self, poll: &mio_06::Poll, token: mio_06::Token, interest: mio_06::Ready, opts: mio_06::PollOpt) -> io::Result<()> {
    self
      .notification_receiver
      .register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &mio_06::Poll,
    token: mio_06::Token,
    interest: mio_06::Ready,
    opts: mio_06::PollOpt,
  ) -> io::Result<()> {
    self
      .notification_receiver
      .reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &mio_06::Poll) -> io::Result<()> {
    self.notification_receiver.deregister(poll)
  }
}


impl<D, DA> mio_08::event::Source for SimpleDataReader<D, DA>
where
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  fn register(&mut self, registry: &mio_08::Registry, token: mio_08::Token, interests: mio_08::Interest) -> io::Result<()>
  {
    self.event_source.register(registry,token, interests)
  }

  fn reregister(&mut self, registry: &mio_08::Registry, token: mio_08::Token, interests: mio_08::Interest) -> io::Result<()>
  {
    self.event_source.reregister(registry, token, interests)
  }

  fn deregister(&mut self, registry: &mio_08::Registry) -> io::Result<()>
  {
    self.event_source.deregister(registry)
  }

}


impl<D, DA> StatusEvented<DataReaderStatus> for SimpleDataReader<D, DA>
where
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  fn as_status_evented(&mut self) -> &dyn Evented {
    self.status_receiver.as_status_evented()
  }

  fn as_status_source(&mut self) -> &mut dyn mio_08::event::Source {
    self.status_receiver.as_status_source()
  }

  fn try_recv_status(&self) -> Option<DataReaderStatus> {
    self.status_receiver.try_recv_status()
  }
}

impl<D, DA> RTPSEntity for SimpleDataReader<D, DA>
where
  D: Keyed + DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  fn guid(&self) -> GUID {
    self.my_guid
  }
}
