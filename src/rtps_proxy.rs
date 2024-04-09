use std::io;

use mio_extras::channel as mio_channel;

use crate::{
  dds::with_key::Sample,
  discovery::{DiscoveredReaderData, DiscoveredWriterData, SpdpDiscoveredParticipantData},
  rtps::Message,
  structure::guid::{EntityId, GuidPrefix},
  DomainParticipant, Keyed,
};
#[cfg(feature = "security")]
use crate::security::{
  ParticipantBuiltinTopicDataSecure, PublicationBuiltinTopicDataSecure,
  SubscriptionBuiltinTopicDataSecure,
};

pub struct ProxyDataEndpoint<T> {
  sender: mio_channel::SyncSender<T>,
  receiver: mio_channel::Receiver<T>,
}

impl<T> ProxyDataEndpoint<T> {
  pub fn try_send(&self, data: T) -> Result<(), mio_channel::TrySendError<T>> {
    self.sender.try_send(data)
  }

  pub fn try_recv_data(&self) -> Option<T> {
    self.receiver.try_recv().ok()
  }
}

impl<T> mio_06::Evented for ProxyDataEndpoint<T> {
  // Call the trait methods of the mio channel which implements Evented
  fn register(
    &self,
    poll: &mio_06::Poll,
    token: mio_06::Token,
    interest: mio_06::Ready,
    opts: mio_06::PollOpt,
  ) -> io::Result<()> {
    self.receiver.register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &mio_06::Poll,
    token: mio_06::Token,
    interest: mio_06::Ready,
    opts: mio_06::PollOpt,
  ) -> io::Result<()> {
    self.receiver.reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &mio_06::Poll) -> io::Result<()> {
    self.receiver.deregister(poll)
  }
}

pub struct ProxyApplicationEndpoints {
  pub rtps: ProxyDataEndpoint<RTPSMessage>,
  pub discovery: ProxyDataEndpoint<DiscoverySample>,
}

pub fn create_connected_proxying_data_endpoints() -> (
  ProxyApplicationEndpoints,
  ProxyDataEndpoint<RTPSMessage>,
  ProxyDataEndpoint<DiscoverySample>,
) {
  let channel_capacity = 128;

  // Channel for Proxy app --> EventLoop (RTPS messages)
  let (app_rtps_sender, evloop_proxy_receiver) =
    mio_channel::sync_channel::<RTPSMessage>(channel_capacity);

  // Channel for EventLoop --> Proxy app (RTPS messages)
  let (evloop_proxy_sender, app_rtps_receiver) =
    mio_channel::sync_channel::<RTPSMessage>(channel_capacity);

  // Endpoints that should end up to EventLoop
  let evloop_endpoints = ProxyDataEndpoint::<RTPSMessage> {
    sender: evloop_proxy_sender,
    receiver: evloop_proxy_receiver,
  };

  // Channel for Proxy app --> Discovery (Discovery samples)
  let (app_discovery_sender, disc_proxy_receiver) =
    mio_channel::sync_channel::<DiscoverySample>(channel_capacity);

  // Channel for Discovery --> Proxy app (Discovery samples)
  let (disc_proxy_sender, app_discovery_receiver) =
    mio_channel::sync_channel::<DiscoverySample>(channel_capacity);

  // Endpoints that should end up to Discovery
  let discovery_endpoints = ProxyDataEndpoint::<DiscoverySample> {
    sender: disc_proxy_sender,
    receiver: disc_proxy_receiver,
  };

  // Endpoints that should end up to the proxy application
  let application_endpoints = ProxyApplicationEndpoints {
    rtps: ProxyDataEndpoint::<RTPSMessage> {
      sender: app_rtps_sender,
      receiver: app_rtps_receiver,
    },
    discovery: ProxyDataEndpoint::<DiscoverySample> {
      sender: app_discovery_sender,
      receiver: app_discovery_receiver,
    },
  };

  (application_endpoints, evloop_endpoints, discovery_endpoints)
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

impl DiscoverySample {
  pub fn sender_participant_guid_prefix(&self) -> GuidPrefix {
    match self {
      Self::Participant(sample) => match sample {
        Sample::Value(data) => data.key().0.prefix,
        Sample::Dispose(guid) => guid.0.prefix,
      },
      Self::Subscription(sample) => match sample {
        Sample::Value(data) => data.key().0.prefix,
        Sample::Dispose(guid) => guid.0.prefix,
      },
      Self::Publication(sample) => match sample {
        Sample::Value(data) => data.key().0.prefix,
        Sample::Dispose(guid) => guid.0.prefix,
      },
      #[cfg(feature = "security")]
      Self::ParticipantSecure(sample) => match sample {
        Sample::Value(data) => data.key().0.prefix,
        Sample::Dispose(guid) => guid.0.prefix,
      },
      #[cfg(feature = "security")]
      Self::SubscriptionSecure(sample) => match sample {
        Sample::Value(data) => data.key().0.prefix,
        Sample::Dispose(guid) => guid.0.prefix,
      },
      #[cfg(feature = "security")]
      Self::PublicationSecure(sample) => match sample {
        Sample::Value(data) => data.key().0.prefix,
        Sample::Dispose(guid) => guid.0.prefix,
      },
    }
  }

  pub fn source_participant_to(&mut self, dp: &DomainParticipant) {
    // Change locators in sample values. Do nothing for dispose samples.
    match self {
      Self::Participant(Sample::Value(dp_data)) => {
        dp_data.metatraffic_unicast_locators = dp.metatraffic_unicast_locators();
        dp_data.metatraffic_multicast_locators = dp.metatraffic_multicast_locators();
        dp_data.default_unicast_locators = dp.user_traffic_unicast_locators();
        dp_data.default_multicast_locators = dp.user_traffic_multicast_locators();
      }
      #[cfg(feature = "security")]
      Self::ParticipantSecure(Sample::Value(data)) => {
        data.participant_data.metatraffic_unicast_locators = dp.metatraffic_unicast_locators();
        data.participant_data.metatraffic_multicast_locators = dp.metatraffic_multicast_locators();
        data.participant_data.default_unicast_locators = dp.user_traffic_unicast_locators();
        data.participant_data.default_multicast_locators = dp.user_traffic_multicast_locators();
      }
      Self::Subscription(Sample::Value(sub_data)) => {
        sub_data.reader_proxy.unicast_locator_list = dp.user_traffic_unicast_locators();
        sub_data.reader_proxy.multicast_locator_list = dp.user_traffic_multicast_locators();
      }
      #[cfg(feature = "security")]
      Self::SubscriptionSecure(Sample::Value(data)) => {
        data
          .discovered_reader_data
          .reader_proxy
          .unicast_locator_list = dp.user_traffic_unicast_locators();
        data
          .discovered_reader_data
          .reader_proxy
          .multicast_locator_list = dp.user_traffic_multicast_locators();
      }
      Self::Publication(Sample::Value(pub_data)) => {
        pub_data.writer_proxy.unicast_locator_list = dp.user_traffic_unicast_locators();
        pub_data.writer_proxy.multicast_locator_list = dp.user_traffic_multicast_locators();
      }
      #[cfg(feature = "security")]
      Self::PublicationSecure(Sample::Value(data)) => {
        data
          .discovered_writer_data
          .writer_proxy
          .unicast_locator_list = dp.user_traffic_unicast_locators();
        data
          .discovered_writer_data
          .writer_proxy
          .multicast_locator_list = dp.user_traffic_multicast_locators();
      }
      Self::Participant(Sample::Dispose(_))
      | Self::Subscription(Sample::Dispose(_))
      | Self::Publication(Sample::Dispose(_)) => {}
      #[cfg(feature = "security")]
      Self::ParticipantSecure(Sample::Dispose(_))
      | Self::SubscriptionSecure(Sample::Dispose(_))
      | Self::PublicationSecure(Sample::Dispose(_)) => {}
    };
  }
}

// Currently contains just the message, we may need something extra in the
// future.
pub struct RTPSMessage {
  pub msg: Message,
}
