use std::{
  collections::HashMap,
  sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
  time::{Duration as StdDuration, Instant},
};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use mio_06::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::{channel as mio_channel, timer::Timer};
use paste::paste; // token pasting macro

use crate::{
  dds::{
    participant::DomainParticipantWeak,
    qos::{
      policy::{
        Deadline, DestinationOrder, Durability, History, Liveliness, Ownership, Presentation,
        PresentationAccessScope, Reliability, TimeBasedFilter,
      },
      QosPolicies, QosPolicyBuilder,
    },
    readcondition::ReadCondition,
    result::{Error, Result},
  },
  discovery::{
    discovery_db::{DiscoveredVia, DiscoveryDB},
    sedp_messages::{
      DiscoveredReaderData, DiscoveredTopicData, DiscoveredWriterData, Endpoint_GUID,
      ParticipantMessageData, ParticipantMessageDataKind, PublicationBuiltinTopicData, ReaderProxy,
      SubscriptionBuiltinTopicData, WriterProxy,
    },
    spdp_participant_data::{Participant_GUID, SpdpDiscoveredParticipantData},
  },
  network::constant::*,
  security::types::*,
  serialization::{
    cdr_deserializer::CDRDeserializerAdapter, cdr_serializer::CDRSerializerAdapter,
    pl_cdr_adapters::*,
  },
  structure::{
    duration::Duration,
    entity::RTPSEntity,
    guid::{EntityId, GuidPrefix, GUID},
    locator::Locator,
    time::Timestamp,
  },
  with_key::{DataReader, DataWriter, Sample},
};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DiscoveryCommand {
  StopDiscovery,
  RemoveLocalWriter {
    guid: GUID,
  },
  RemoveLocalReader {
    guid: GUID,
  },
  ManualAssertLiveliness,
  AssertTopicLiveliness {
    writer_guid: GUID,
    manual_assertion: bool,
  },
}

pub struct LivelinessState {
  last_auto_update: Timestamp,
  last_manual_participant_update: Timestamp,
}

impl LivelinessState {
  pub fn new() -> Self {
    Self {
      last_auto_update: Timestamp::now(),
      last_manual_participant_update: Timestamp::now(),
    }
  }
}

// TODO: Refactor this. Maybe the repeating groups of "topic", "reader",
// "writer", "timer" below could be abstracted to a common struct:

type DataReaderPlCdr<D> = DataReader<D, PlCdrDeserializerAdapter<D>>;
type DataWriterPlCdr<D> = DataWriter<D, PlCdrSerializerAdapter<D>>;

mod with_key {
  use serde::{de::DeserializeOwned, Serialize};
  use mio_extras::timer::Timer;

  use super::{DataReaderPlCdr, DataWriterPlCdr};
  use crate::{serialization::pl_cdr_adapters::*, Key, Keyed, Topic, TopicKind};

  pub const TOPIC_KIND: TopicKind = TopicKind::WithKey;

  pub(super) struct DiscoveryTopicPlCdr<D>
  where
    D: Keyed + PlCdrSerialize + PlCdrDeserialize,
    <D as Keyed>::K: Key + PlCdrSerialize + PlCdrDeserialize,
  {
    #[allow(dead_code)] // The topic may not be accesssed after initialization
    pub topic: Topic,
    pub reader: DataReaderPlCdr<D>,
    pub writer: DataWriterPlCdr<D>,
    pub timer: Timer<()>,
  }

  pub(super) struct DiscoveryTopicCDR<D>
  where
    D: Keyed + Serialize + DeserializeOwned,
    <D as Keyed>::K: Key + Serialize + DeserializeOwned,
  {
    #[allow(dead_code)] // The topic may not be accesssed after initialization
    pub topic: Topic,
    pub reader: crate::with_key::DataReaderCdr<D>,
    pub writer: crate::with_key::DataWriterCdr<D>,
    pub timer: Timer<()>,
  }
}

mod no_key {
  use serde::{de::DeserializeOwned, Serialize};
  use mio_extras::timer::Timer;

  use crate::{Topic, TopicKind};

  pub const TOPIC_KIND: TopicKind = TopicKind::NoKey;

  pub(super) struct DiscoveryTopicCDR<D>
  where
    D: Serialize + DeserializeOwned,
  {
    #[allow(dead_code)] // The topic may not be accesssed after initialization
    pub topic: Topic,
    pub reader: crate::no_key::DataReader<D, crate::CDRDeserializerAdapter<D>>,
    pub writer: crate::no_key::DataWriter<D, crate::CDRSerializerAdapter<D>>,
    pub timer: Timer<()>,
  }
}

pub(crate) struct Discovery {
  poll: Poll,
  domain_participant: DomainParticipantWeak,
  discovery_db: Arc<RwLock<DiscoveryDB>>,

  // Discovery started sender confirms to application thread that we are running
  discovery_started_sender: std::sync::mpsc::Sender<Result<()>>,
  // notification sender goes to dp_event_loop thread
  discovery_updated_sender: mio_channel::SyncSender<DiscoveryNotificationType>,
  // Discovery gets commands from dp_event_loop from this channel
  discovery_command_receiver: mio_channel::Receiver<DiscoveryCommand>,
  spdp_liveness_receiver: mio_channel::Receiver<GuidPrefix>,

  liveliness_state: LivelinessState,

  // TODO: Why is this a HashMap? Are there ever more than 2?
  self_locators: HashMap<Token, Vec<Locator>>,

  // DDS Subsciber and Publisher for Discovery
  // ...but these are not actually used after initialization
  //discovery_subscriber: Subscriber,
  //discovery_publisher: Publisher,

  // Handling of "DCPSParticipant" topic. This is the mother of all topics
  // where participants announce their presence and built-in readers and writers.
  // and
  // timer to periodically announce our presence
  dcps_participant: with_key::DiscoveryTopicPlCdr<SpdpDiscoveredParticipantData>,
  participant_cleanup_timer: Timer<()>, // garbage collection timer for dead remote particiapnts

  // Topic "DCPSSubscription" - announcing and detecting Readers
  dcps_subscription: with_key::DiscoveryTopicPlCdr<DiscoveredReaderData>,

  // Topic "DCPSPublication" - announcing and detecting Writers
  dcps_publication: with_key::DiscoveryTopicPlCdr<DiscoveredWriterData>,

  // Topic "DCPSTopic" - annoncing and detecting topics
  #[allow(dead_code)] // Technically, the topic is not accesssed after initialization
  dcps_topic: with_key::DiscoveryTopicPlCdr<DiscoveredTopicData>,
  topic_cleanup_timer: Timer<()>,

  // DCPSParticipantMessage - used by participants to communicate liveness
  dcps_participant_message: with_key::DiscoveryTopicCDR<ParticipantMessageData>,

  // Following topics from DDS Security spec v1.1

  // DCPSParticipantSecure - 7.4.1.6 New DCPSParticipantSecure Builtin Topic
  #[allow(dead_code)] // TODO: Remove when handlers implemented
  dcps_participant_secure: with_key::DiscoveryTopicPlCdr<ParticipantBuiltinTopicDataSecure>,

  // DCPSPublicationsSecure - 7.4.1.7 New DCPSPublicationsSecure Builtin Topic
  #[allow(dead_code)] // TODO: Remove when handlers implemented
  dcps_publications_secure: with_key::DiscoveryTopicPlCdr<PublicationBuiltinTopicDataSecure>,

  // DCPSSubscriptionsSecure - 7.4.1.8 New DCPSSubscriptionsSecure Builtin Topic
  #[allow(dead_code)] // TODO: Remove when handlers implemented
  dcps_subscriptions_secure: with_key::DiscoveryTopicPlCdr<SubscriptionBuiltinTopicDataSecure>,

  // DCPSParticipantMessageSecure - used by participants to communicate secure liveness
  // 7.4.2 New DCPSParticipantMessageSecure builtin Topic
  #[allow(dead_code)] // TODO: Remove when handlers implemented
  dcps_participant_message_secure: with_key::DiscoveryTopicCDR<ParticipantMessageData>, /* CDR, not PL_CDR */

  // DCPSParticipantStatelessMessageSecure
  // 77.4.3 New DCPSParticipantStatelessMessage builtin Topic
  // !!! TODO: By the spec, this topic must use _stateless_ reader and writer, which are
  // insensitive to sequence number attacks.
  #[allow(dead_code)] // TODO: Remove when handlers implemented
  dcps_participant_stateless_message: no_key::DiscoveryTopicCDR<ParticipantStatelessMessage>,

  // DCPSParticipantVolatileMessageSecure
  // 7.4.4 New DCPSParticipantVolatileMessageSecure builtin Topic
  #[allow(dead_code)] // TODO: Remove when handlers implemented
  dcps_participant_volatile_message_secure:
    no_key::DiscoveryTopicCDR<ParticipantVolatileMessageSecure>, // CDR?
}

impl Discovery {
  const PARTICIPANT_CLEANUP_PERIOD: StdDuration = StdDuration::from_secs(2);
  const TOPIC_CLEANUP_PERIOD: StdDuration = StdDuration::from_secs(60); // timer for cleaning up inactive topics
  const SEND_PARTICIPANT_INFO_PERIOD: StdDuration = StdDuration::from_secs(2);
  const SEND_READERS_INFO_PERIOD: StdDuration = StdDuration::from_secs(2);
  const SEND_WRITERS_INFO_PERIOD: StdDuration = StdDuration::from_secs(2);
  const SEND_TOPIC_INFO_PERIOD: StdDuration = StdDuration::from_secs(10);
  const CHECK_PARTICIPANT_MESSAGES: StdDuration = StdDuration::from_secs(1);

  pub(crate) const PARTICIPANT_MESSAGE_QOS: QosPolicies = QosPolicies {
    durability: Some(Durability::TransientLocal),
    presentation: None,
    deadline: None,
    latency_budget: None,
    ownership: None,
    liveliness: None,
    time_based_filter: None,
    reliability: Some(Reliability::Reliable {
      max_blocking_time: Duration::DURATION_ZERO,
    }),
    destination_order: None,
    history: Some(History::KeepLast { depth: 1 }),
    resource_limits: None,
    lifespan: None,
  };

  pub fn new(
    domain_participant: DomainParticipantWeak,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    discovery_started_sender: std::sync::mpsc::Sender<Result<()>>,
    discovery_updated_sender: mio_channel::SyncSender<DiscoveryNotificationType>,
    discovery_command_receiver: mio_channel::Receiver<DiscoveryCommand>,
    spdp_liveness_receiver: mio_channel::Receiver<GuidPrefix>,
    self_locators: HashMap<Token, Vec<Locator>>,
  ) -> Result<Self> {
    macro_rules! try_construct {
      ($constructor:expr, $msg:literal) => {
        match $constructor {
          Ok(r) => r,
          Err(e) => {
            error!($msg, e);
            discovery_started_sender
              .send(Err(Error::OutOfResources))
              .unwrap_or(()); // We are trying to quit. If send fails, just ignore it.
            return Err(Error::OutOfResources);
          }
        }
      };
    }

    let poll = try_construct!(
      mio_06::Poll::new(),
      "Failed to allocate discovery poll. {:?}"
    );
    let discovery_subscriber_qos = Self::subscriber_qos();
    let discovery_publisher_qos = Self::publisher_qos();

    // Create DDS Publisher and Subscriber for Discovery.
    // These are needed to create DataWriter and DataReader objects
    let discovery_subscriber = try_construct!(
      domain_participant.create_subscriber(&discovery_subscriber_qos),
      "Unable to create Discovery Subscriber. {:?}"
    );
    let discovery_publisher = try_construct!(
      domain_participant.create_publisher(&discovery_publisher_qos),
      "Unable to create Discovery Publisher. {:?}"
    );

    macro_rules! construct_topic_and_poll {
      ( $repr:ident, $has_key:ident,
        $topic_name:expr, $topic_type_name:expr, $message_type:ty,
        $qos:expr,
        $reader_entity_id:expr, $reader_token:expr,
        $writer_entity_id:expr,
        $timeout:expr, $timer_token:expr, ) => {{
        let topic = domain_participant
          .create_topic(
            $topic_name.to_string(),
            $topic_type_name.to_string(),
            &discovery_subscriber_qos,
            $has_key::TOPIC_KIND,
          )
          .expect("Unable to create topic. ");
        paste! {
          let reader =
            discovery_subscriber
            . [< create_datareader_with_entityid_ $has_key >]
              ::<$message_type, [<$repr DeserializerAdapter>] <$message_type>>(
              &topic,
              $reader_entity_id,
              $qos,
            ).expect("Unable to create DataReader. ");

          let writer =
              discovery_publisher.[< create_datawriter_with_entityid_ $has_key >]
                ::<$message_type, [<$repr SerializerAdapter>] <$message_type>>(
                $writer_entity_id,
                &topic,
                $qos,
              ).expect("Unable to create DataWriter .");
        }
        poll
          .register(&reader, $reader_token, Ready::readable(), PollOpt::edge())
          .expect("Failed to register a discovery reader to poll.");

        let mut timer: Timer<()> = Timer::default();
        timer.set_timeout($timeout, ());
        poll
          .register(&timer, $timer_token, Ready::readable(), PollOpt::edge())
          .expect("Unable to register timer token. ");

        paste! { $has_key ::[<DiscoveryTopic $repr>] { topic, reader, writer, timer } }
      }}; // macro
    }

    try_construct!(
      poll.register(
        &discovery_command_receiver,
        DISCOVERY_COMMAND_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      ),
      "Failed to register Discovery poll. {:?}"
    );

    try_construct!(
      poll.register(
        &spdp_liveness_receiver,
        SPDP_LIVENESS_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      ),
      "Failed to register Discovery poll. {:?}"
    );

    // Participant
    let dcps_participant = construct_topic_and_poll!(
      PlCdr,
      with_key,
      "DCPSParticipant",               // topic name
      "SPDPDiscoveredParticipantData", // topic type name over RTPS
      SpdpDiscoveredParticipantData,
      Some(Self::create_spdp_patricipant_qos()),
      EntityId::SPDP_BUILTIN_PARTICIPANT_READER,
      DISCOVERY_PARTICIPANT_DATA_TOKEN,
      EntityId::SPDP_BUILTIN_PARTICIPANT_WRITER,
      Self::SEND_PARTICIPANT_INFO_PERIOD,
      DISCOVERY_SEND_PARTICIPANT_INFO_TOKEN,
    );

    // create lease duration check timer
    let mut participant_cleanup_timer: Timer<()> = Timer::default();
    participant_cleanup_timer.set_timeout(Self::PARTICIPANT_CLEANUP_PERIOD, ());
    try_construct!(
      poll.register(
        &participant_cleanup_timer,
        DISCOVERY_PARTICIPANT_CLEANUP_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      ),
      "Unable to create participant cleanup timer. {:?}"
    );

    // Subscriptions: What are the Readers on the network and what are they
    // subscribing to?
    let dcps_subscription = construct_topic_and_poll!(
      PlCdr,
      with_key,
      "DCPSSubscription",     // topic name
      "DiscoveredReaderData", // topic type name over RTPS
      DiscoveredReaderData,
      None, // QoS
      EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_READER,
      DISCOVERY_READER_DATA_TOKEN,
      EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_WRITER,
      Self::SEND_READERS_INFO_PERIOD,
      DISCOVERY_SEND_READERS_INFO_TOKEN,
    );

    // Publication : Who are the Writers here and elsewhere
    let dcps_publication = construct_topic_and_poll!(
      PlCdr,
      with_key,
      "DCPSPublication",      // topic name
      "DiscoveredReaderData", // topic type name over RTPS
      DiscoveredWriterData,
      None, // QoS
      EntityId::SEDP_BUILTIN_PUBLICATIONS_READER,
      DISCOVERY_WRITER_DATA_TOKEN,
      EntityId::SEDP_BUILTIN_PUBLICATIONS_WRITER,
      Self::SEND_WRITERS_INFO_PERIOD,
      DISCOVERY_SEND_WRITERS_INFO_TOKEN,
    );

    // Topic topic (not a typo)
    let dcps_topic = construct_topic_and_poll!(
      PlCdr,
      with_key,
      "DCPSTopic",           // topic name
      "DiscoveredTopicData", // topic type name over RTPS
      DiscoveredTopicData,
      None, // QoS
      EntityId::SEDP_BUILTIN_TOPIC_READER,
      DISCOVERY_TOPIC_DATA_TOKEN,
      EntityId::SEDP_BUILTIN_TOPIC_WRITER,
      Self::SEND_TOPIC_INFO_PERIOD,
      DISCOVERY_SEND_TOPIC_INFO_TOKEN,
    );

    // create lease duration check timer
    let mut topic_cleanup_timer: Timer<()> = Timer::default();
    topic_cleanup_timer.set_timeout(Self::TOPIC_CLEANUP_PERIOD, ());
    try_construct!(
      poll.register(
        &topic_cleanup_timer,
        DISCOVERY_TOPIC_CLEANUP_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      ),
      "Unable to register topic cleanup timer. {:?}"
    );

    // Participant Message Data 8.4.13
    let dcps_participant_message = construct_topic_and_poll!(
      CDR,
      with_key,
      "DCPSParticipantMessage", // topic name
      "ParticipantMessageData", // topic type name over RTPS
      ParticipantMessageData,
      Some(Self::PARTICIPANT_MESSAGE_QOS),
      EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_READER,
      DISCOVERY_PARTICIPANT_MESSAGE_TOKEN,
      EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
      Self::CHECK_PARTICIPANT_MESSAGES,
      DISCOVERY_PARTICIPANT_MESSAGE_TIMER_TOKEN,
    );

    // DDS Security

    // Participant
    let dcps_participant_secure = construct_topic_and_poll!(
      PlCdr,
      with_key,
      "DCPSParticipantsSecure",            // topic name
      "ParticipantBuiltinTopicDataSecure", // topic type name over RTPS (use the same data type)
      ParticipantBuiltinTopicDataSecure,
      None, // QoS
      EntityId::SPDP_RELIABLE_BUILTIN_PARTICIPANT_SECURE_READER,
      SECURE_DISCOVERY_PARTICIPANT_DATA_TOKEN,
      EntityId::SPDP_RELIABLE_BUILTIN_PARTICIPANT_SECURE_WRITER,
      Self::SEND_PARTICIPANT_INFO_PERIOD,
      SECURE_DISCOVERY_SEND_PARTICIPANT_INFO_TOKEN,
    );

    // Subscriptions: What are the Readers on the network and what are they
    // subscribing to?
    let dcps_subscriptions_secure = construct_topic_and_poll!(
      PlCdr,
      with_key,
      "DCPSSubscriptionsSecure",            // topic name
      "SubscriptionBuiltinTopicDataSecure", // topic type name over RTPS
      SubscriptionBuiltinTopicDataSecure,
      None, // QoS
      EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_SECURE_READER,
      SECURE_DISCOVERY_READER_DATA_TOKEN,
      EntityId::SEDP_BUILTIN_SUBSCRIPTIONS_SECURE_WRITER,
      Self::SEND_READERS_INFO_PERIOD,
      SECURE_DISCOVERY_SEND_READERS_INFO_TOKEN,
    );

    // Publication : Who are the Writers here and elsewhere
    let dcps_publications_secure = construct_topic_and_poll!(
      PlCdr,
      with_key,
      "DCPSPublicationsSecure",             // topic name
      "PublicationBuiltinTopicDataSecure,", // topic type name over RTPS
      PublicationBuiltinTopicDataSecure,
      None, // QoS
      EntityId::SEDP_BUILTIN_PUBLICATIONS_SECURE_READER,
      SECURE_DISCOVERY_WRITER_DATA_TOKEN,
      EntityId::SEDP_BUILTIN_PUBLICATIONS_SECURE_WRITER,
      Self::SEND_WRITERS_INFO_PERIOD,
      SECURE_DISCOVERY_SEND_WRITERS_INFO_TOKEN,
    );

    // p2p Participant message secure
    let dcps_participant_message_secure = construct_topic_and_poll!(
      CDR,
      with_key,
      "DCPSParticipantMessageSecure", // topic name
      "ParticipantMessageData",       // topic type name over RTPS (use the same data type)
      ParticipantMessageData,         // actually reuse the non-secure data type
      None,                           // QoS
      EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_SECURE_READER,
      P2P_SECURE_DISCOVERY_PARTICIPANT_MESSAGE_TOKEN,
      EntityId::P2P_BUILTIN_PARTICIPANT_MESSAGE_SECURE_WRITER,
      Self::CHECK_PARTICIPANT_MESSAGES,
      P2P_SECURE_DISCOVERY_PARTICIPANT_MESSAGE_TIMER_TOKEN,
    );
    //TODO: NO_KEY topic
    let dcps_participant_stateless_message = construct_topic_and_poll!(
      CDR,
      no_key,
      "DCPSParticipantStatelessMessage", // topic name
      "ParticipantStatelessMessage",
      ParticipantStatelessMessage,
      None, // QoS
      EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_READER,
      P2P_PARTICIPANT_STATELESS_MESSAGE_TOKEN,
      EntityId::P2P_BUILTIN_PARTICIPANT_STATELESS_WRITER,
      Self::CHECK_PARTICIPANT_MESSAGES,
      P2P_PARTICIPANT_STATELESS_MESSAGE_TIMER_TOKEN,
    );
    //TODO: NO_KEY topic
    let dcps_participant_volatile_message_secure = construct_topic_and_poll!(
      CDR,
      no_key,
      "ParticipantVolatileMessageSecure", // topic name
      "ParticipantVolatileMessageSecure",
      ParticipantVolatileMessageSecure, // actually reuse the non-secure data type
      None,                             // QoS
      EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_READER,
      P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_TOKEN,
      EntityId::P2P_BUILTIN_PARTICIPANT_VOLATILE_SECURE_WRITER,
      Self::CHECK_PARTICIPANT_MESSAGES,
      P2P_BUILTIN_PARTICIPANT_VOLATILE_TIMER_TOKEN,
    );

    Ok(Self {
      poll,
      domain_participant,
      discovery_db,
      discovery_started_sender,
      discovery_updated_sender,
      discovery_command_receiver,
      spdp_liveness_receiver,
      self_locators,

      liveliness_state: LivelinessState::new(),

      // discovery_subscriber,
      // discovery_publisher,
      dcps_participant,
      participant_cleanup_timer, // SPDP
      dcps_subscription,
      dcps_publication, // SEDP
      dcps_topic,
      topic_cleanup_timer,      // SEDP
      dcps_participant_message, // liveliness messages

      dcps_participant_secure,
      dcps_publications_secure,
      dcps_subscriptions_secure,
      dcps_participant_message_secure,
      dcps_participant_stateless_message,
      dcps_participant_volatile_message_secure,
    })
  }

  pub fn discovery_event_loop(&mut self) {
    self.initialize_participant();

    // send out info about non-built-in Writers and Readers that we have.
    self.write_writers_info();
    self.write_readers_info();

    match self.discovery_started_sender.send(Ok(())) {
      Ok(_) => (),
      _ => return, // Participant has probably crashed at this point
    };

    loop {
      let mut events = Events::with_capacity(32); // Should this be outside of the loop?
      match self.poll.poll(&mut events, None) {
        Ok(_) => (),
        Err(e) => {
          error!("Failed in waiting of poll in discovery. {e:?}");
          return;
        }
      }

      for event in events.into_iter() {
        match event.token() {
          DISCOVERY_COMMAND_TOKEN => {
            while let Ok(command) = self.discovery_command_receiver.try_recv() {
              match command {
                DiscoveryCommand::StopDiscovery => {
                  info!("Stopping Discovery");
                  // disposing readers
                  let db = self.discovery_db_read();
                  for reader in db.get_all_local_topic_readers() {
                    self
                      .dcps_subscription
                      .writer
                      .dispose(&Endpoint_GUID(reader.reader_proxy.remote_reader_guid), None)
                      .unwrap_or(());
                  }

                  for writer in db.get_all_local_topic_writers() {
                    self
                      .dcps_publication
                      .writer
                      .dispose(&Endpoint_GUID(writer.writer_proxy.remote_writer_guid), None)
                      .unwrap_or(());
                  }
                  // finally disposing the participant we have
                  self
                    .dcps_participant
                    .writer
                    .dispose(&Participant_GUID(self.domain_participant.guid()), None)
                    .unwrap_or(());
                  info!("Stopped Discovery");
                  return; // terminate event loop
                }
                DiscoveryCommand::RemoveLocalWriter { guid } => {
                  if guid == self.dcps_publication.writer.guid() {
                    continue;
                  }
                  self
                    .dcps_publication
                    .writer
                    .dispose(&Endpoint_GUID(guid), None)
                    .unwrap_or_else(|e| error!("Disposing local Writer: {e:?}"));

                  match self.discovery_db.write() {
                    Ok(mut db) => db.remove_local_topic_writer(guid),
                    Err(e) => {
                      error!("DiscoveryDB is poisoned. {e:?}");
                      return;
                    }
                  }
                }
                DiscoveryCommand::RemoveLocalReader { guid } => {
                  if guid == self.dcps_subscription.writer.guid() {
                    continue;
                  }

                  self
                    .dcps_subscription
                    .writer
                    .dispose(&Endpoint_GUID(guid), None)
                    .unwrap_or_else(|e| error!("Disposing local Reader: {e:?}"));

                  match self.discovery_db.write() {
                    Ok(mut db) => db.remove_local_topic_reader(guid),
                    Err(e) => {
                      error!("DiscoveryDB is poisoned. {e:?}");
                      return;
                    }
                  }
                }
                DiscoveryCommand::ManualAssertLiveliness => {
                  self.liveliness_state.last_manual_participant_update = Timestamp::now();
                }
                DiscoveryCommand::AssertTopicLiveliness {
                  writer_guid,
                  manual_assertion,
                } => {
                  self.send_discovery_notification(
                    DiscoveryNotificationType::AssertTopicLiveliness {
                      writer_guid,
                      manual_assertion,
                    },
                  );
                }
              };
            }
          }

          DISCOVERY_PARTICIPANT_DATA_TOKEN => {
            debug!("triggered participant reader");
            self.handle_participant_reader();
          }

          DISCOVERY_PARTICIPANT_CLEANUP_TOKEN => {
            self.participant_cleanup();
            // setting next cleanup timeout
            self
              .participant_cleanup_timer
              .set_timeout(Self::PARTICIPANT_CLEANUP_PERIOD, ());
          }

          DISCOVERY_SEND_PARTICIPANT_INFO_TOKEN => {
            let strong_dp = if let Some(dp) = self.domain_participant.clone().upgrade() {
              dp
            } else {
              error!("DomainParticipant doesn't exist anymore, exiting Discovery.");
              return;
            };

            // setting 5 times the duration so lease doesn't break if update fails once or
            // twice
            let data = SpdpDiscoveredParticipantData::from_local_participant(
              &strong_dp,
              &self.self_locators,
              5.0 * Duration::from(Self::SEND_PARTICIPANT_INFO_PERIOD),
            );

            self
              .dcps_participant
              .writer
              .write(data, None)
              .unwrap_or_else(|e| {
                error!("Discovery: Publishing to DCPS participant topic failed: {e:?}");
              });
            // reschedule timer
            self
              .dcps_participant
              .timer
              .set_timeout(Self::SEND_PARTICIPANT_INFO_PERIOD, ());
          }
          DISCOVERY_READER_DATA_TOKEN => {
            self.handle_subscription_reader(None);
          }
          DISCOVERY_SEND_READERS_INFO_TOKEN => {
            self.write_readers_info();
            self
              .dcps_subscription
              .timer
              .set_timeout(Self::SEND_READERS_INFO_PERIOD, ());
          }
          DISCOVERY_WRITER_DATA_TOKEN => {
            self.handle_publication_reader(None);
          }
          DISCOVERY_SEND_WRITERS_INFO_TOKEN => {
            self.write_writers_info();
            self
              .dcps_publication
              .timer
              //              .writers_send_info_timer
              .set_timeout(Self::SEND_WRITERS_INFO_PERIOD, ());
          }
          DISCOVERY_TOPIC_DATA_TOKEN => {
            self.handle_topic_reader(None);
          }
          DISCOVERY_TOPIC_CLEANUP_TOKEN => {
            self.topic_cleanup();

            self
              .topic_cleanup_timer
              .set_timeout(Self::TOPIC_CLEANUP_PERIOD, ());
          }
          DISCOVERY_SEND_TOPIC_INFO_TOKEN => {
            self.write_topic_info();
            self
              .dcps_topic
              .timer
              //.topic_info_send_timer
              .set_timeout(Self::SEND_TOPIC_INFO_PERIOD, ());
          }
          DISCOVERY_PARTICIPANT_MESSAGE_TOKEN => {
            self.handle_participant_message_reader();
          }
          DISCOVERY_PARTICIPANT_MESSAGE_TIMER_TOKEN => {
            self.write_participant_message();
            self
              .dcps_participant_message
              .timer
              .set_timeout(Self::CHECK_PARTICIPANT_MESSAGES, ());
          }
          SPDP_LIVENESS_TOKEN => {
            while let Ok(guid_prefix) = self.spdp_liveness_receiver.try_recv() {
              match self.discovery_db.write() {
                Ok(mut db) => db.participant_is_alive(guid_prefix),
                Err(e) => {
                  error!("DiscoveryDB is poisoned. {e:?}");
                  return;
                }
              }
            }
          }
          other_token => {
            error!("discovery event loop got token: {:?}", other_token);
          }
        } // match
      } // for
    } // loop
  } // fn

  // Initialize our own particiapnt data into the Discovery DB.
  // That causes ReaderProxies and WriterProxies to be constructed and
  // and we also get our own local readers and writers connected, both
  // built-in and user-defined.
  // If we did not do this, the Readers and Writers in this participant could not
  // find each other.
  fn initialize_participant(&self) {
    let dp = if let Some(dp) = self.domain_participant.clone().upgrade() {
      dp
    } else {
      error!("Cannot get actual DomainParticipant in initialize_participant! Giving up.");
      return;
    };

    let participant_data = SpdpDiscoveredParticipantData::from_local_participant(
      &dp,
      &self.self_locators,
      Duration::DURATION_INFINITE,
    );

    // Initialize our own particiapnt data into the Discovery DB, so we can talk to
    // ourself.
    self
      .discovery_db_write()
      .update_participant(&participant_data);

    // This will read the participant from Discovery DB and construct
    // ReaderProxy and WriterProxy objects for built-in Readers and Writers
    self.send_discovery_notification(DiscoveryNotificationType::ParticipantUpdated {
      guid_prefix: dp.guid().prefix,
    });

    // insert a (fake) reader proxy as multicast address, so discovery notifications
    // are sent somewhere
    let reader_guid = GUID::new(
      GuidPrefix::UNKNOWN,
      EntityId::SPDP_BUILTIN_PARTICIPANT_READER,
    );

    // Do we expect inlineQos in every incoming DATA message?
    let rustdds_expects_inline_qos = false;

    let reader_proxy = ReaderProxy::new(
      reader_guid,
      rustdds_expects_inline_qos,
      self
        .self_locators
        .get(&DISCOVERY_LISTENER_TOKEN)
        .cloned()
        .unwrap_or_default(),
      self
        .self_locators
        .get(&DISCOVERY_MUL_LISTENER_TOKEN)
        .cloned()
        .unwrap_or_default(),
    );

    let sub_topic_data = SubscriptionBuiltinTopicData::new(
      reader_guid,
      Some(dp.guid()),
      String::from("DCPSParticipant"),
      String::from("SPDPDiscoveredParticipantData"),
      &Self::create_spdp_patricipant_qos(),
      None, // <<---------------TODO: None here means we advertise no EndpointSecurityInfo
    );
    let drd = DiscoveredReaderData {
      reader_proxy,
      subscription_topic_data: sub_topic_data,
      content_filter: None,
    };

    let writer_guid = GUID::new(dp.guid().prefix, EntityId::SPDP_BUILTIN_PARTICIPANT_WRITER);

    let writer_proxy = WriterProxy::new(
      writer_guid,
      self
        .self_locators
        .get(&DISCOVERY_LISTENER_TOKEN)
        .cloned()
        .unwrap_or_default(),
      self
        .self_locators
        .get(&DISCOVERY_MUL_LISTENER_TOKEN)
        .cloned()
        .unwrap_or_default(),
    );

    let pub_topic_data = PublicationBuiltinTopicData::new(
      writer_guid,
      Some(dp.guid()),
      String::from("DCPSParticipant"),
      String::from("SPDPDiscoveredParticipantData"),
      None, // TODO: EndpointSecurityInfo is missing from here.
    );
    let dwd = DiscoveredWriterData {
      last_updated: Instant::now(),
      writer_proxy,
      publication_topic_data: pub_topic_data,
    };

    // Notify local Readers and Writers in dp_event_loop
    // so that they will create WriterProxies and ReaderProxies
    // and know to communicate with them.
    info!("Creating DCPSParticipant reader proxy.");
    self.send_discovery_notification(DiscoveryNotificationType::ReaderUpdated {
      discovered_reader_data: drd,
    });
    info!("Creating DCPSParticipant writer proxy for self.");
    self.send_discovery_notification(DiscoveryNotificationType::WriterUpdated {
      discovered_writer_data: dwd,
    });
  }

  pub fn handle_participant_reader(&mut self) {
    loop {
      let s = self.dcps_participant.reader.take_next_sample();
      debug!("handle_participant_reader read {:?}", &s);
      match s {
        Ok(Some(d)) => match d.value {
          Sample::Value(participant_data) => {
            debug!(
              "handle_participant_reader discovered {:?}",
              &participant_data
            );
            let was_new = self
              .discovery_db_write()
              .update_participant(&participant_data);
            let guid_prefix = participant_data.participant_guid.prefix;
            self.send_discovery_notification(DiscoveryNotificationType::ParticipantUpdated {
              guid_prefix,
            });
            if was_new {
              // This may be a rediscovery of a previously seen participant that
              // was temporarily lost due to network outage. Check if we already know
              // what it has (readers, writers, topics).
              debug!("Participant rediscovery start");
              self.handle_topic_reader(Some(guid_prefix));
              self.handle_subscription_reader(Some(guid_prefix));
              self.handle_publication_reader(Some(guid_prefix));
              debug!("Participant rediscovery finished");
            }
          }
          // Sample::Dispose means that DomainParticipant was disposed
          Sample::Dispose(participant_guid) => {
            self
              .discovery_db_write()
              .remove_participant(participant_guid.0.prefix, true); // true = actively removed
            self.send_discovery_notification(DiscoveryNotificationType::ParticipantLost {
              guid_prefix: participant_guid.0.prefix,
            });
          }
        },
        Ok(None) => {
          trace!("handle_participant_reader: no more data");
          return;
        } // no more data
        Err(e) => {
          error!(" !!! handle_participant_reader: {e:?}");
          return;
        }
      }
    } // loop
  }

  // Check if there are messages about new Readers
  pub fn handle_subscription_reader(&mut self, read_history: Option<GuidPrefix>) {
    let drds: Vec<Sample<DiscoveredReaderData, GUID>> =
      match self.dcps_subscription.reader.into_iterator() {
        Ok(ds) => ds
          .map(|d| d.map_dispose(|g| g.0)) // map_dispose removes Endpoint_GUID wrapper around GUID
          .filter(|d|
              // If a particiapnt was specified, we must match its GUID prefix.
              match (read_history, d) {
                (None, _) => true, // Not asked to filter by participant
                (Some(participant_to_update), Sample::Value(drd)) =>
                  drd.reader_proxy.remote_reader_guid.prefix == participant_to_update,
                (Some(participant_to_update), Sample::Dispose(guid)) =>
                  guid.prefix == participant_to_update,
              })
          .collect(),
        Err(e) => {
          error!("handle_subscription_reader: {e:?}");
          return;
        }
      };

    for d in drds {
      match d {
        Sample::Value(d) => {
          let drd = self.discovery_db_write().update_subscription(&d);
          debug!(
            "handle_subscription_reader - send_discovery_notification ReaderUpdated  {:?}",
            &drd
          );
          self.send_discovery_notification(DiscoveryNotificationType::ReaderUpdated {
            discovered_reader_data: drd,
          });
          if read_history.is_some() {
            info!(
              "Rediscovered reader {:?} topic={:?}",
              d.reader_proxy.remote_reader_guid,
              d.subscription_topic_data.topic_name()
            );
          }
        }
        Sample::Dispose(reader_key) => {
          info!("Dispose Reader {:?}", reader_key);
          self.discovery_db_write().remove_topic_reader(reader_key);
          self.send_discovery_notification(DiscoveryNotificationType::ReaderLost {
            reader_guid: reader_key,
          });
        }
      }
    } // loop
  }

  pub fn handle_publication_reader(&mut self, read_history: Option<GuidPrefix>) {
    let dwds: Vec<Sample<DiscoveredWriterData, GUID>> =
      match self.dcps_publication.reader.into_iterator() {
        // a lot of cloning here, but we must copy the data out of the
        // reader before we can use self again, as .read() returns references to within
        // a reader and thus self
        Ok(ds) => ds
          .map(|d| d.map_dispose(|g| g.0)) // map_dispose removes Endpoint_GUID wrapper around GUID
          // If a particiapnt was specified, we must match its GUID prefix.
          .filter(|d| match (read_history, d) {
            (None, _) => true, // Not asked to filter by participant
            (Some(participant_to_update), Sample::Value(dwd)) => {
              dwd.writer_proxy.remote_writer_guid.prefix == participant_to_update
            }
            (Some(participant_to_update), Sample::Dispose(guid)) => {
              guid.prefix == participant_to_update
            }
          })
          .collect(),
        Err(e) => {
          error!("handle_publication_reader: {e:?}");
          return;
        }
      };

    for d in dwds {
      match d {
        Sample::Value(dwd) => {
          trace!("handle_publication_reader discovered {:?}", &dwd);
          let discovered_writer_data = self.discovery_db_write().update_publication(&dwd);
          self.send_discovery_notification(DiscoveryNotificationType::WriterUpdated {
            discovered_writer_data,
          });
          debug!("Discovered Writer {:?}", &dwd);
        }
        Sample::Dispose(writer_key) => {
          self.discovery_db_write().remove_topic_writer(writer_key);
          self.send_discovery_notification(DiscoveryNotificationType::WriterLost {
            writer_guid: writer_key,
          });
          debug!("Disposed Writer {:?}", writer_key);
        }
      }
    } // loop
  }

  // TODO: Try to remember why the read_history parameter below was introduced
  // in the first place. Git history should help here.
  // Likely it is something to do with an unreliable network and
  // DomainParticipants timing out and then coming back. The read_history was
  // supposed to help in recovering from that.
  pub fn handle_topic_reader(&mut self, _read_history: Option<GuidPrefix>) {
    let ts: Vec<Sample<(DiscoveredTopicData, GUID), GUID>> = match self
      .dcps_topic
      .reader
      .take(usize::MAX, ReadCondition::any())
    {
      Ok(ds) => ds
        .iter()
        .map(|d| {
          d.value
            .clone()
            .map_value(|o| (o, d.sample_info.writer_guid()))
            .map_dispose(|g| g.0)
        })
        .collect(),
      Err(e) => {
        error!("handle_topic_reader: {e:?}");
        return;
      }
    };

    for t in ts {
      match t {
        Sample::Value((topic_data, writer)) => {
          info!("handle_topic_reader discovered {:?}", &topic_data);
          self
            .discovery_db_write()
            .update_topic_data(&topic_data, writer, DiscoveredVia::Topic);
          // Now check if we know any readers of writers to this topic. The topic QoS
          // could cause these to became viable matches agains local
          // writers/readers. This is because at least RTI Connext sends QoS
          // policies on a Topic, and then (apprently) assumes that its
          // readers/writers inherit those policies unless specified otherwise.

          let writers = self
            .discovery_db_read()
            .writers_on_topic_and_participant(topic_data.topic_name(), writer.prefix);
          info!("writers {:?}", &writers);
          for discovered_writer_data in writers {
            self.send_discovery_notification(DiscoveryNotificationType::WriterUpdated {
              discovered_writer_data,
            });
          }

          let readers = self
            .discovery_db_read()
            .readers_on_topic_and_participant(topic_data.topic_name(), writer.prefix);
          for discovered_reader_data in readers {
            self.send_discovery_notification(DiscoveryNotificationType::ReaderUpdated {
              discovered_reader_data,
            });
          }
        }
        // Sample::Dispose means disposed
        Sample::Dispose(key) => {
          warn!("not implemented - Topic was disposed: {:?}", &key);
        }
      }
    } // loop
  }

  // These messages are for updating participant liveliness
  // The protocol distinguises between automatic (by DDS library)
  // and manual (by by application, via DDS API call) liveness
  // TODO: rewrite this function according to the pattern above
  pub fn handle_participant_message_reader(&mut self) {
    let participant_messages: Option<Vec<ParticipantMessageData>> = match self
      .dcps_participant_message
      .reader
      .take(100, ReadCondition::any())
    {
      Ok(msgs) => Some(
        msgs
          .into_iter()
          .filter_map(|p| p.value().clone().value())
          .collect(),
      ),
      _ => None,
    };

    let msgs = match participant_messages {
      Some(d) => d,
      None => return,
    };

    let mut db = self.discovery_db_write();
    for msg in msgs.into_iter() {
      db.update_lease_duration(&msg);
    }
  }

  // TODO: Explain what happens here and by what logic
  pub fn write_participant_message(&mut self) {
    let writer_liveliness: Vec<Liveliness> = self
      .discovery_db_read()
      .get_all_local_topic_writers()
      .filter_map(|p| {
        let liveliness = match p.publication_topic_data.liveliness {
          Some(lv) => lv,
          None => return None,
        };

        Some(liveliness)
      })
      .collect();

    let (automatic, manual): (Vec<&Liveliness>, Vec<&Liveliness>) =
      writer_liveliness.iter().partition(|p| match p {
        Liveliness::Automatic { lease_duration: _ } => true,
        Liveliness::ManualByParticipant { lease_duration: _ } => false,
        Liveliness::ManualByTopic { lease_duration: _ } => false,
      });

    let (manual_by_participant, _manual_by_topic): (Vec<&Liveliness>, Vec<&Liveliness>) =
      manual.iter().partition(|p| match p {
        Liveliness::Automatic { lease_duration: _ } => false,
        Liveliness::ManualByParticipant { lease_duration: _ } => true,
        Liveliness::ManualByTopic { lease_duration: _ } => false,
      });

    let inow = Timestamp::now();

    // Automatic
    {
      let current_duration = inow.duration_since(self.liveliness_state.last_auto_update) / 3;
      let min_automatic = automatic
        .iter()
        .map(|lv| match lv {
          Liveliness::Automatic { lease_duration }
          | Liveliness::ManualByParticipant { lease_duration }
          | Liveliness::ManualByTopic { lease_duration } => lease_duration,
        })
        .min();
      trace!(
        "Current auto duration {:?}. Min auto duration {:?}",
        current_duration,
        min_automatic
      );
      if let Some(&mm) = min_automatic {
        if current_duration > mm {
          let pp = ParticipantMessageData {
            guid: self.domain_participant.guid_prefix(),
            kind: ParticipantMessageDataKind::AUTOMATIC_LIVELINESS_UPDATE,
            data: Vec::new(),
          };
          match self.dcps_participant_message.writer.write(pp, None) {
            Ok(_) => (),
            Err(e) => {
              error!("Failed to write ParticipantMessageData auto. {e:?}");
              return;
            }
          }
          self.liveliness_state.last_auto_update = inow;
        }
      };
    }

    // Manual By Participant
    {
      let current_duration =
        inow.duration_since(self.liveliness_state.last_manual_participant_update) / 3;
      let min_manual_participant = manual_by_participant
        .iter()
        .map(|lv| match lv {
          Liveliness::Automatic { lease_duration }
          | Liveliness::ManualByParticipant { lease_duration }
          | Liveliness::ManualByTopic { lease_duration } => lease_duration,
        })
        .min();
      if let Some(&dur) = min_manual_participant {
        if current_duration > dur {
          let pp = ParticipantMessageData {
            guid: self.domain_participant.guid_prefix(),
            kind: ParticipantMessageDataKind::MANUAL_LIVELINESS_UPDATE,
            data: Vec::new(),
          };
          match self.dcps_participant_message.writer.write(pp, None) {
            Ok(_) => (),
            Err(e) => {
              error!("Failed to writer ParticipantMessageData manual. {e:?}");
            }
          }
        }
      };
    }
  }

  pub fn participant_cleanup(&self) {
    let removed_guid_prefixes = self.discovery_db_write().participant_cleanup();
    for guid_prefix in removed_guid_prefixes {
      debug!("participant cleanup - timeout for {:?}", guid_prefix);
      self.send_discovery_notification(DiscoveryNotificationType::ParticipantLost { guid_prefix });
    }
  }

  pub fn topic_cleanup(&self) {
    self.discovery_db_write().topic_cleanup();
  }

  pub fn write_readers_info(&self) {
    let db = self.discovery_db_read();
    let local_user_readers = db.get_all_local_topic_readers().filter(|p| {
      p.reader_proxy
        .remote_reader_guid
        .entity_id
        .kind()
        .is_user_defined()
    });
    let mut count = 0;
    for data in local_user_readers {
      match self.dcps_subscription.writer.write(data.clone(), None) {
        Ok(_) => {
          count += 1;
        }
        Err(e) => error!("Unable to write new readers info. {e:?}"),
      }
    }
    debug!("Announced {} readers", count);
  }

  pub fn write_writers_info(&self) {
    let db = self.discovery_db_read();
    let local_user_writers = db.get_all_local_topic_writers().filter(|p| {
      p.writer_proxy
        .remote_writer_guid
        .entity_id
        .kind()
        .is_user_defined()
    });
    let mut count = 0;
    for data in local_user_writers {
      if self
        .dcps_publication
        .writer
        .write(data.clone(), None)
        .is_err()
      {
        error!("Unable to write new writers info.");
      } else {
        count += 1;
      }
    }
    debug!("Announced {} writers", count);
  }

  pub fn write_topic_info(&self) {
    let db = self.discovery_db_read();
    let datas = db.local_user_topics();
    for data in datas {
      if let Err(e) = self.dcps_topic.writer.write(data.clone(), None) {
        error!("Unable to write new topic info: {e:?}");
      }
    }
  }

  pub fn subscriber_qos() -> QosPolicies {
    QosPolicyBuilder::new()
      .durability(Durability::TransientLocal)
      .presentation(Presentation {
        access_scope: PresentationAccessScope::Topic,
        coherent_access: false,
        ordered_access: false,
      })
      .deadline(Deadline(Duration::DURATION_INFINITE))
      .ownership(Ownership::Shared)
      .liveliness(Liveliness::Automatic {
        lease_duration: Duration::DURATION_INFINITE,
      })
      .time_based_filter(TimeBasedFilter {
        minimum_separation: Duration::DURATION_ZERO,
      })
      .reliability(Reliability::Reliable {
        max_blocking_time: Duration::from_std(StdDuration::from_millis(100)),
      })
      .destination_order(DestinationOrder::ByReceptionTimestamp)
      .history(History::KeepLast { depth: 10 })
      // .resource_limits(ResourceLimits { // TODO: Maybe lower limits would suffice?
      //   max_instances: std::i32::MAX,
      //   max_samples: std::i32::MAX,
      //   max_samples_per_instance: std::i32::MAX,
      // })
      .build()
  }

  // TODO: Check if this definition is correct (spec?)
  pub fn publisher_qos() -> QosPolicies {
    QosPolicyBuilder::new()
      .durability(Durability::TransientLocal)
      .presentation(Presentation {
        access_scope: PresentationAccessScope::Topic,
        coherent_access: false,
        ordered_access: false,
      })
      .deadline(Deadline(Duration::DURATION_INFINITE))
      .ownership(Ownership::Shared)
      .liveliness(Liveliness::Automatic {
        lease_duration: Duration::DURATION_INFINITE,
      })
      .time_based_filter(TimeBasedFilter {
        minimum_separation: Duration::DURATION_ZERO,
      })
      .reliability(Reliability::Reliable {
        max_blocking_time: Duration::from_std(StdDuration::from_millis(100)),
      })
      .destination_order(DestinationOrder::ByReceptionTimestamp)
      .history(History::KeepLast { depth: 10 })
      // .resource_limits(ResourceLimits { // TODO: Maybe lower limits would suffice?
      //   max_instances: std::i32::MAX,
      //   max_samples: std::i32::MAX,
      //   max_samples_per_instance: std::i32::MAX,
      // })
      .build()
  }

  pub fn create_spdp_patricipant_qos() -> QosPolicies {
    QosPolicyBuilder::new()
      .reliability(Reliability::BestEffort)
      .history(History::KeepLast { depth: 1 })
      .build()
  }

  fn discovery_db_read(&self) -> RwLockReadGuard<DiscoveryDB> {
    match self.discovery_db.read() {
      Ok(db) => db,
      Err(e) => panic!("DiscoveryDB is poisoned {:?}.", e),
    }
  }

  fn discovery_db_write(&self) -> RwLockWriteGuard<DiscoveryDB> {
    match self.discovery_db.write() {
      Ok(db) => db,
      Err(e) => panic!("DiscoveryDB is poisoned {:?}.", e),
    }
  }

  fn send_discovery_notification(&self, dntype: DiscoveryNotificationType) {
    match self.discovery_updated_sender.send(dntype) {
      Ok(_) => (),
      Err(e) => error!("Failed to send DiscoveryNotification {e:?}"),
    }
  }
}

// -----------------------------------------------------------------------
// -----------------------------------------------------------------------
// -----------------------------------------------------------------------
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::net::SocketAddr;

  use chrono::Utc;
  //use bytes::Bytes;
  use mio_06::Token;
  use speedy::{Endianness, Writable};

  use super::*;
  use crate::{
    dds::{adapters::no_key::DeserializerAdapter, participant::DomainParticipant},
    discovery::sedp_messages::TopicBuiltinTopicData,
    messages::submessages::submessages::{InterpreterSubmessage, WriterSubmessage},
    network::{udp_listener::UDPListener, udp_sender::UDPSender},
    rtps::submessage::*,
    serialization::cdr_deserializer::CDRDeserializerAdapter,
    structure::{entity::RTPSEntity, locator::Locator},
    test::{
      shape_type::ShapeType,
      test_data::{
        create_cdr_pl_rtps_data_message, spdp_participant_msg_mod, spdp_publication_msg,
        spdp_subscription_msg,
      },
    },
    RepresentationIdentifier,
  };

  #[test]
  fn discovery_participant_data_test() {
    let poll = Poll::new().unwrap();
    let mut udp_listener = UDPListener::new_unicast("127.0.0.1", 11000).unwrap();
    poll
      .register(
        udp_listener.mio_socket(),
        Token(0),
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    // sending participant data to discovery
    let udp_sender = UDPSender::new_with_random_port().expect("failed to create UDPSender");
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      spdp_well_known_unicast_port(0, 0),
    )];

    let tdata = spdp_participant_msg_mod(11000);
    let msg_data = tdata
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .expect("Failed to write msg data");

    udp_sender.send_to_all(&msg_data, &addresses);

    let mut events = Events::with_capacity(10);
    poll
      .poll(&mut events, Some(StdDuration::from_secs(1)))
      .unwrap();

    let _data2 = udp_listener.get_message();
    // TODO: we should have received our own participants info decoding the
    // actual message might be good idea
  }

  #[test]
  fn discovery_reader_data_test() {
    use crate::{serialization::pl_cdr_adapters::PlCdrSerialize, TopicKind};

    let participant = DomainParticipant::new(0).expect("participant creation");

    let topic = participant
      .create_topic(
        "Square".to_string(),
        "ShapeType".to_string(),
        &QosPolicies::qos_none(),
        TopicKind::WithKey,
      )
      .unwrap();

    let publisher = participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let _writer = publisher
      .create_datawriter_cdr::<ShapeType>(&topic, None)
      .unwrap();

    let subscriber = participant
      .create_subscriber(&QosPolicies::qos_none())
      .unwrap();
    let _reader =
      subscriber.create_datareader::<ShapeType, CDRDeserializerAdapter<ShapeType>>(&topic, None);

    let poll = Poll::new().unwrap();
    let mut udp_listener = UDPListener::new_unicast("127.0.0.1", 11001).unwrap();
    poll
      .register(
        udp_listener.mio_socket(),
        Token(0),
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    let udp_sender = UDPSender::new_with_random_port().expect("failed to create UDPSender");
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      spdp_well_known_unicast_port(14, 0),
    )];

    let mut tdata = spdp_subscription_msg();
    let mut data;
    for submsg in &mut tdata.submessages {
      match &mut submsg.body {
        SubmessageBody::Writer(v) => match v {
          WriterSubmessage::Data(d, _) => {
            let mut drd: DiscoveredReaderData = PlCdrDeserializerAdapter::from_bytes(
              &d.serialized_payload.as_ref().unwrap().value,
              RepresentationIdentifier::PL_CDR_LE,
            )
            .unwrap();
            drd.reader_proxy.unicast_locator_list.clear();
            drd
              .reader_proxy
              .unicast_locator_list
              .push(Locator::from(SocketAddr::new(
                "127.0.0.1".parse().unwrap(),
                11001,
              )));
            drd.reader_proxy.multicast_locator_list.clear();

            data = drd
              .to_pl_cdr_bytes(RepresentationIdentifier::PL_CDR_LE)
              .unwrap();
            d.serialized_payload.as_mut().unwrap().value = data.clone();
          }
          _ => continue,
        },
        SubmessageBody::Interpreter(_) => (),
        _ => continue,
      }
    }

    let msg_data = tdata
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .expect("Failed to write msg dtaa");

    udp_sender.send_to_all(&msg_data, &addresses);

    let mut events = Events::with_capacity(10);
    poll
      .poll(&mut events, Some(StdDuration::from_secs(1)))
      .unwrap();

    let _data2 = udp_listener.get_message();
  }

  #[test]
  fn discovery_writer_data_test() {
    use crate::TopicKind;
    let participant = DomainParticipant::new(0).expect("Failed to create participant");

    let topic = participant
      .create_topic(
        "Square".to_string(),
        "ShapeType".to_string(),
        &QosPolicies::qos_none(),
        TopicKind::WithKey,
      )
      .unwrap();

    let publisher = participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let _writer = publisher
      .create_datawriter_cdr::<ShapeType>(&topic, None)
      .unwrap();

    let subscriber = participant
      .create_subscriber(&QosPolicies::qos_none())
      .unwrap();
    let _reader =
      subscriber.create_datareader::<ShapeType, CDRDeserializerAdapter<ShapeType>>(&topic, None);

    let poll = Poll::new().unwrap();
    let mut udp_listener = UDPListener::new_unicast("127.0.0.1", 0).unwrap();
    poll
      .register(
        udp_listener.mio_socket(),
        Token(0),
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    let udp_sender = UDPSender::new_with_random_port().expect("failed to create UDPSender");
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      spdp_well_known_unicast_port(15, 0),
    )];

    let mut tdata = spdp_publication_msg();
    for submsg in &mut tdata.submessages {
      match &mut submsg.body {
        SubmessageBody::Interpreter(v) => match v {
          InterpreterSubmessage::InfoDestination(dst, _flags) => {
            dst.guid_prefix = participant.guid_prefix();
          }
          _ => continue,
        },
        SubmessageBody::Writer(_) => (),
        SubmessageBody::Reader(_) => (),
        SubmessageBody::Security(_) => (),
      }
    }

    let par_msg_data = spdp_participant_msg_mod(udp_listener.port())
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .expect("Failed to write participant data.");

    let msg_data = tdata
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .expect("Failed to write msg data");

    udp_sender.send_to_all(&par_msg_data, &addresses);
    udp_sender.send_to_all(&msg_data, &addresses);

    let mut events = Events::with_capacity(10);
    poll
      .poll(&mut events, Some(StdDuration::from_secs(1)))
      .unwrap();

    for _ in udp_listener.messages() {
      info!("Message received");
    }
  }

  #[test]
  fn discovery_topic_data_test() {
    let _participant = DomainParticipant::new(0);

    let topic_data = DiscoveredTopicData::new(
      Utc::now(),
      TopicBuiltinTopicData {
        key: None,
        name: String::from("Square"),
        type_name: String::from("ShapeType"),
        durability: None,
        deadline: None,
        latency_budget: None,
        liveliness: None,
        reliability: None,
        lifespan: None,
        destination_order: None,
        presentation: None,
        history: None,
        resource_limits: None,
        ownership: None,
      },
    );

    let rtps_message = create_cdr_pl_rtps_data_message(
      topic_data,
      EntityId::SEDP_BUILTIN_TOPIC_READER,
      EntityId::SEDP_BUILTIN_TOPIC_WRITER,
    );

    let udp_sender = UDPSender::new_with_random_port().expect("failed to create UDPSender");
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      spdp_well_known_unicast_port(16, 0),
    )];

    let rr = rtps_message
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .unwrap();

    udp_sender.send_to_all(&rr, &addresses);
  }
}
