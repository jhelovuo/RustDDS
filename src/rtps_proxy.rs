use std::{
  io,
  sync::{Arc, Mutex},
};

use mio_06::event::Evented;
use mio_extras::channel as mio_channel;

use crate::{
  dds::{participant::DomainParticipantDisc, with_key::Sample},
  discovery::{DiscoveredReaderData, DiscoveredWriterData, SpdpDiscoveredParticipantData},
  rtps::Message,
  structure::guid::EntityId,
  Keyed,
};
#[cfg(feature = "security")]
use crate::security::{
  ParticipantBuiltinTopicDataSecure, PublicationBuiltinTopicDataSecure,
  SubscriptionBuiltinTopicDataSecure,
};

pub trait ProxyEvented {
  fn as_proxy_evented(&mut self) -> &dyn Evented; // This is for polling with mio-0.6.x
  fn try_recv_data(&self) -> Option<ProxyData>;
}

#[derive(Clone)]
pub struct ProxyDataChannelSender {
  actual_sender: mio_channel::SyncSender<ProxyData>,
}

impl ProxyDataChannelSender {
  pub fn try_send(&self, data: ProxyData) -> Result<(), mio_channel::TrySendError<ProxyData>> {
    self.actual_sender.try_send(data)
  }
}

pub struct ProxyDataChannelReceiver {
  actual_receiver: Mutex<mio_channel::Receiver<ProxyData>>,
}

impl mio_06::Evented for ProxyDataChannelReceiver {
  // Call the trait methods of the mio channel which implements Evented
  fn register(
    &self,
    poll: &mio_06::Poll,
    token: mio_06::Token,
    interest: mio_06::Ready,
    opts: mio_06::PollOpt,
  ) -> io::Result<()> {
    self
      .actual_receiver
      .lock()
      .unwrap()
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
      .actual_receiver
      .lock()
      .unwrap()
      .reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &mio_06::Poll) -> io::Result<()> {
    self.actual_receiver.lock().unwrap().deregister(poll)
  }
}

impl ProxyEvented for ProxyDataChannelReceiver {
  fn as_proxy_evented(&mut self) -> &dyn Evented {
    self
  }
  fn try_recv_data(&self) -> Option<ProxyData> {
    self.actual_receiver.lock().unwrap().try_recv().ok()
  }
}

pub(crate) fn sync_proxy_data_channel(
  capacity: usize,
) -> io::Result<(ProxyDataChannelSender, ProxyDataChannelReceiver)> {
  let (actual_sender, actual_receiver) = mio_channel::sync_channel(capacity);
  Ok((
    ProxyDataChannelSender { actual_sender },
    ProxyDataChannelReceiver {
      actual_receiver: Mutex::new(actual_receiver),
    },
  ))
}

// Entity IDs used in those discovery topics that contain locator data.
// RTPS messages cannot be directly proxied because of the locator data.
pub const ENTITY_IDS_WITH_NO_DIRECT_RTPS_PROXYING: &[EntityId] = &[
  EntityId::SPDP_BUILTIN_PARTICIPANT_READER,
  EntityId::SPDP_BUILTIN_PARTICIPANT_WRITER,
  EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_READER,
  EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_WRITER,
  EntityId::SEDP_BUILTIN_PUBLICATIONS_READER,
  EntityId::SEDP_BUILTIN_PUBLICATIONS_WRITER,
  #[cfg(feature = "security")]
  EntityId::SPDP_RELIABLE_BUILTIN_PARTICIPANT_SECURE_READER,
  #[cfg(feature = "security")]
  EntityId::SPDP_RELIABLE_BUILTIN_PARTICIPANT_SECURE_WRITER,
  #[cfg(feature = "security")]
  EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_SECURE_READER,
  #[cfg(feature = "security")]
  EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_SECURE_WRITER,
  #[cfg(feature = "security")]
  EntityId::SEDP_BUILTIN_PUBLICATIONS_SECURE_READER,
  #[cfg(feature = "security")]
  EntityId::SEDP_BUILTIN_PUBLICATIONS_SECURE_WRITER,
];

#[derive(Clone, Debug)]
// Samples of discovery data that contains locators
pub enum DiscoverySample {
  Participant(Sample<SpdpDiscoveredParticipantData, <SpdpDiscoveredParticipantData as Keyed>::K>),
  Subscription(Sample<DiscoveredReaderData, <DiscoveredReaderData as Keyed>::K>),
  Publication(Sample<DiscoveredWriterData, <DiscoveredWriterData as Keyed>::K>),
  #[cfg(feature = "security")]
  ParticipantSecure(
    Sample<ParticipantBuiltinTopicDataSecure, <ParticipantBuiltinTopicDataSecure as Keyed>::K>,
  ),
  #[cfg(feature = "security")]
  SubscriptionSecure(
    Sample<SubscriptionBuiltinTopicDataSecure, <SubscriptionBuiltinTopicDataSecure as Keyed>::K>,
  ),
  #[cfg(feature = "security")]
  PublicationSecure(
    Sample<PublicationBuiltinTopicDataSecure, <PublicationBuiltinTopicDataSecure as Keyed>::K>,
  ),
}

#[allow(clippy::large_enum_variant)] // Temporary, for dev only
#[derive(Clone, Debug)]
pub enum ProxyData {
  RTPSMessage(Message),
  Discovery(DiscoverySample),
}

pub struct ProxyDataListener {
  pub(crate) dp_disc: Arc<Mutex<DomainParticipantDisc>>,
}

impl ProxyEvented for ProxyDataListener {
  fn as_proxy_evented(&mut self) -> &dyn Evented {
    self
  }
  fn try_recv_data(&self) -> Option<ProxyData> {
    self
      .dp_disc
      .lock()
      .unwrap()
      .proxy_channel_receiver()
      .try_recv_data()
  }
}

impl mio_06::Evented for ProxyDataListener {
  // Call the Evented trait methods of the ProxyDataChannelReceiver in the
  // DomainParticipant
  fn register(
    &self,
    poll: &mio_06::Poll,
    token: mio_06::Token,
    interest: mio_06::Ready,
    opts: mio_06::PollOpt,
  ) -> io::Result<()> {
    self
      .dp_disc
      .lock()
      .unwrap()
      .proxy_channel_receiver_mut()
      .as_proxy_evented()
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
      .dp_disc
      .lock()
      .unwrap()
      .proxy_channel_receiver_mut()
      .as_proxy_evented()
      .reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &mio_06::Poll) -> io::Result<()> {
    self
      .dp_disc
      .lock()
      .unwrap()
      .proxy_channel_receiver_mut()
      .as_proxy_evented()
      .deregister(poll)
  }
}
