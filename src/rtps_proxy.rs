use std::{
  io,
  sync::{Arc, Mutex},
};

use mio_06::event::Evented;
use mio_extras::channel as mio_channel;

use crate::{
  dds::participant::DomainParticipantDisc,
  discovery::{DiscoveredReaderData, DiscoveredWriterData, SpdpDiscoveredParticipantData},
  rtps::Message,
};
#[cfg(feature = "security")]
use crate::security::{
  ParticipantBuiltinTopicDataSecure, PublicationBuiltinTopicDataSecure,
  SubscriptionBuiltinTopicDataSecure,
};

pub trait ProxyEvented {
  fn as_proxy_evented(&mut self) -> &dyn Evented; // This is for polling with mio-0.6.x
}

#[derive(Clone)]
pub struct ProxyDataChannelSender {
  actual_sender: mio_channel::SyncSender<ProxyData>,
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

#[derive(Clone)]
pub enum DiscoveryDataWithLocator {
  Participant(SpdpDiscoveredParticipantData),
  Subscription(DiscoveredReaderData),
  Publication(DiscoveredWriterData),
  #[cfg(feature = "security")]
  ParticipantSecure(ParticipantBuiltinTopicDataSecure),
  #[cfg(feature = "security")]
  SubscriptionSecure(SubscriptionBuiltinTopicDataSecure),
  #[cfg(feature = "security")]
  PublicationSecure(PublicationBuiltinTopicDataSecure),
}

#[allow(clippy::large_enum_variant)] // Temporary, for dev only
#[derive(Clone)]
pub enum ProxyData {
  RTPSMessage(Message),
  Discovery(DiscoveryDataWithLocator),
}

pub struct ProxyDataListener {
  pub(crate) dp_disc: Arc<Mutex<DomainParticipantDisc>>,
}

impl ProxyEvented for ProxyDataListener {
  fn as_proxy_evented(&mut self) -> &dyn Evented {
    self
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
