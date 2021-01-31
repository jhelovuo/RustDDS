
use crate::discovery::data_types::topic_data::ReaderProxy;
use crate::network::util::get_local_multicast_locators;
use crate::dds::data_types::SubscriptionBuiltinTopicData;
use crate::dds::rtps_reader_proxy::RtpsReaderProxy;
#[allow(unused_imports)] use log::{debug, error, info,trace};

use mio::{Ready, Poll, PollOpt, Events};
use mio_extras::timer::Timer;
use mio_extras::channel as mio_channel;

use std::{
  sync::{Arc, RwLock},
  sync::RwLockReadGuard,
  sync::RwLockWriteGuard,
  time::Duration as StdDuration,
};

use crate::{
  dds::{
    with_key::datareader::DataReader, with_key::datareader::DataReader_CDR,
    with_key::datawriter::DataWriter,
    topic::*,
    participant::{DomainParticipantWeak},
    qos::{
      QosPolicies,
      policy::{
        Reliability, History, Durability, Presentation, PresentationAccessScope, Deadline,
        Ownership, Liveliness, TimeBasedFilter, DestinationOrder,
      },
    },
    readcondition::ReadCondition,
  },
  dds::values::result::Error,
  structure::entity::RTPSEntity,
  structure::guid::{ GUID, GuidPrefix, },
  dds::qos::QosPolicyBuilder,
};

use crate::discovery::{
  data_types::spdp_participant_data::SPDPDiscoveredParticipantData,
  data_types::topic_data::{DiscoveredWriterData, DiscoveredReaderData},
  discovery_db::DiscoveryDB,
};

use crate::structure::{duration::Duration, guid::EntityId, time::Timestamp};

use crate::serialization::{CDRSerializerAdapter, pl_cdr_deserializer::PlCdrDeserializerAdapter};

use crate::network::constant::*;
use super::data_types::topic_data::{
  DiscoveredTopicData, ParticipantMessageData, ParticipantMessageDataKind,
};
use byteorder::LittleEndian;

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DiscoveryCommand {
  STOP_DISCOVERY,
  REMOVE_LOCAL_WRITER { guid: GUID },
  REMOVE_LOCAL_READER { guid: GUID },
  MANUAL_ASSERT_LIVELINESS,
  ASSERT_TOPIC_LIVELINESS { writer_guid: GUID , manual_assertion: bool, },
}

pub struct LivelinessState {
  last_auto_update: Timestamp,
  last_manual_participant_update: Timestamp,
}

impl LivelinessState {
  pub fn new() -> LivelinessState {
    LivelinessState {
      last_auto_update: Timestamp::now(),
      last_manual_participant_update: Timestamp::now(),
    }
  }
}

pub(crate) struct Discovery {
  poll: Poll,
  domain_participant: DomainParticipantWeak,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  discovery_started_sender: std::sync::mpsc::Sender<Result<(), Error>>,
  discovery_updated_sender: mio_channel::SyncSender<DiscoveryNotificationType>,
  discovery_command_receiver: mio_channel::Receiver<DiscoveryCommand>,
}

impl Discovery {
  const PARTICIPANT_CLEANUP_PERIOD: StdDuration = StdDuration::from_secs(2);
  const TOPIC_CLEANUP_PERIOD: StdDuration = StdDuration::from_secs(10); // timer for cleaning up inactive topics
  const SEND_PARTICIPANT_INFO_PERIOD: StdDuration = StdDuration::from_secs(2);
  const SEND_READERS_INFO_PERIOD: StdDuration = StdDuration::from_secs(2);
  const SEND_WRITERS_INFO_PERIOD: StdDuration = StdDuration::from_secs(2);
  const SEND_TOPIC_INFO_PERIOD: StdDuration = StdDuration::from_secs(20);
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
    discovery_started_sender: std::sync::mpsc::Sender<Result<(), Error>>,
    discovery_updated_sender: mio_channel::SyncSender<DiscoveryNotificationType>,
    discovery_command_receiver: mio_channel::Receiver<DiscoveryCommand>,
  ) -> Discovery {
    let poll = mio::Poll::new()
      .unwrap_or_else(|e| {
        error!("Failed to allocate discovery poll. {:?}", e);
        discovery_started_sender
          .send(Err(Error::OutOfResources))
          .unwrap_or(());
        panic!("Failed to allocate discovery poll. {:?}", e);
      });

    Discovery {
      poll,
      domain_participant,
      discovery_db,
      discovery_started_sender,
      discovery_updated_sender,
      discovery_command_receiver,
    }
  }

  fn create_spdp_patricipant_qos() -> QosPolicies {
    QosPolicyBuilder::new()
      .reliability(Reliability::BestEffort)
      .history(History::KeepLast { depth: 1 })
      .build()
  }

  pub fn discovery_event_loop(discovery: Discovery) {

    macro_rules! try_construct {
      ($constructor:expr, $msg:literal) => (
        match $constructor {
          Ok(r) => r,
          Err(e) => {
            error!($msg, e);
            discovery.discovery_started_sender.send(Err(Error::OutOfResources))
              .unwrap_or(()); // We are trying to quit. If send fails, just ignore it.
            return
          }
        }
      )
    }

    let mut liveliness_state = LivelinessState::new();

    try_construct!( discovery.poll.register(
        &discovery.discovery_command_receiver,
        DISCOVERY_COMMAND_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      ),
      "Failed to register Discovery STOP. {:?}");

    let discovery_subscriber_qos = Discovery::subscriber_qos();

    let discovery_subscriber = try_construct!(
      discovery.domain_participant.create_subscriber(&discovery_subscriber_qos),
      "Unable to create Discovery Subscriber. {:?}"
    );

    let discovery_publisher_qos = Discovery::subscriber_qos();
    let discovery_publisher = try_construct!(
      discovery.domain_participant.create_publisher(&discovery_publisher_qos) ,
      "Unable to create Discovery Publisher. {:?}");

    // Participant
    let dcps_participant_topic = try_construct!(
      discovery.domain_participant.create_topic(
        "DCPSParticipant",
        "SPDPDiscoveredParticipantData",
        &Discovery::create_spdp_patricipant_qos(),
        TopicKind::WithKey,
      ),
      "Unable to create DCPSParticipant topic. {:?}");
  
    let mut dcps_participant_reader = try_construct!( discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData,PlCdrDeserializerAdapter<SPDPDiscoveredParticipantData>>(
        dcps_participant_topic.clone(),
        Some(EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER),
        None,
      ) ,"Unable to create DataReader for DCPSParticipant. {:?}");

    // register participant reader
    try_construct!( discovery.poll.register(
      &dcps_participant_reader,
      DISCOVERY_PARTICIPANT_DATA_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Failed to register participant reader to poll. {:?}");

    // create lease duration check timer
    let mut participant_cleanup_timer: Timer<()> = Timer::default();
    participant_cleanup_timer.set_timeout(Discovery::PARTICIPANT_CLEANUP_PERIOD, ());
    try_construct!( discovery.poll.register(
      &participant_cleanup_timer,
      DISCOVERY_PARTICIPANT_CLEANUP_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to create participant cleanup timer. {:?}");

    let dcps_participant_writer = try_construct!( discovery_publisher
      .create_datawriter_CDR::<SPDPDiscoveredParticipantData>(
        Some(EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER),
        dcps_participant_topic,
        None,
      ) ,"Unable to create DataWriter for DCPSParticipant. {:?}");

    // creating timer for sending out own participant data
    let mut participant_send_info_timer: Timer<()> = Timer::default();
    participant_send_info_timer.set_timeout(Discovery::SEND_PARTICIPANT_INFO_PERIOD, ());

    try_construct!( discovery.poll.register(
      &participant_send_info_timer,
      DISCOVERY_SEND_PARTICIPANT_INFO_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to register participant info sender. {:?}");

    // Subcription
    let dcps_subscription_qos = Discovery::subscriber_qos();
    let dcps_subscription_topic = try_construct!( discovery.domain_participant.create_topic(
      "DCPSSubscription",
      "DiscoveredReaderData",
      &dcps_subscription_qos,
      TopicKind::WithKey,
    ) ,"Unable to create DCPSSubscription topic. {:?}");

    let mut dcps_subscription_reader = try_construct!( discovery_subscriber
      .create_datareader::<DiscoveredReaderData, PlCdrDeserializerAdapter<DiscoveredReaderData>>(
        dcps_subscription_topic.clone(),
        Some(EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER),
        None,
      ) ,"Unable to create DataReader for DCPSSubscription. {:?}");

    try_construct!( discovery.poll.register(
      &dcps_subscription_reader,
      DISCOVERY_READER_DATA_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to register subscription reader. {:?}");

    let mut dcps_subscription_writer = try_construct!( discovery_publisher
      .create_datawriter_CDR::<DiscoveredReaderData>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER),
        dcps_subscription_topic,
        None,
      ) ,"Unable to create DataWriter for DCPSSubscription. {:?}");

    let mut readers_send_info_timer: Timer<()> = Timer::default();
    readers_send_info_timer.set_timeout(Discovery::SEND_READERS_INFO_PERIOD, ());
    try_construct!( discovery.poll.register(
      &readers_send_info_timer,
      DISCOVERY_SEND_READERS_INFO_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to register readers info sender. {:?}");

    // Publication
    let dcps_publication_qos = Discovery::subscriber_qos();
    let dcps_publication_topic = try_construct!( discovery.domain_participant.create_topic(
      "DCPSPublication",
      "DiscoveredWriterData",
      &dcps_publication_qos,
      TopicKind::WithKey,
    ) ,"Unable to create DCPSPublication topic. {:?}");

    let mut dcps_publication_reader = try_construct!( discovery_subscriber
      .create_datareader::<DiscoveredWriterData, PlCdrDeserializerAdapter<DiscoveredWriterData>>(
        dcps_publication_topic.clone(),
        Some(EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER),
        None,
      ) ,"Unable to create DataReader for DCPSPublication. {:?}");

    try_construct!( discovery.poll.register(
      &dcps_publication_reader,
      DISCOVERY_WRITER_DATA_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to regiser writers info sender. {:?}");

    let mut dcps_publication_writer = try_construct!( discovery_publisher
      .create_datawriter_CDR::<DiscoveredWriterData>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER),
        dcps_publication_topic,
        None,
      ) ,"Unable to create DataWriter for DCPSPublication. {:?}");

    let mut writers_send_info_timer: Timer<()> = Timer::default();
    writers_send_info_timer.set_timeout(Discovery::SEND_WRITERS_INFO_PERIOD, ());
    try_construct!( discovery.poll.register(
      &writers_send_info_timer,
      DISCOVERY_SEND_WRITERS_INFO_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to register readers info sender. {:?}");

    // Topic
    let dcps_topic_qos = QosPolicyBuilder::new().build();
    let dcps_topic = try_construct!( discovery.domain_participant.create_topic(
      "DCPSTopic",
      "DiscoveredTopicData",
      &dcps_topic_qos,
      TopicKind::WithKey,
    ) ,"Unable to create DCPSTopic topic. {:?}");

    // create lease duration check timer
    let mut topic_cleanup_timer: Timer<()> = Timer::default();
    topic_cleanup_timer.set_timeout(Discovery::TOPIC_CLEANUP_PERIOD, ());
    try_construct!( discovery.poll.register(
      &topic_cleanup_timer,
      DISCOVERY_TOPIC_CLEANUP_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to register topic cleanup timer. {:?}");

    let mut dcps_reader = try_construct!( discovery_subscriber
      .create_datareader::<DiscoveredTopicData, PlCdrDeserializerAdapter<DiscoveredTopicData>>(
        dcps_topic.clone(),
        Some(EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER),
        None,
      ) ,"Unable to create DataReader for DCPSTopic. {:?}");

    try_construct!( discovery.poll.register(
      &dcps_reader,
      DISCOVERY_TOPIC_DATA_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to register topic reader. {:?}");

    let mut dcps_writer = try_construct!( discovery_publisher
      .create_datawriter_CDR::<DiscoveredTopicData>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER),
        dcps_topic,
        None,
      ) ,"Unable to create DataWriter for DCPSTopic. {:?}");

    let mut topic_info_send_timer: Timer<()> = Timer::default();
    topic_info_send_timer.set_timeout(Discovery::SEND_TOPIC_INFO_PERIOD, ());
    try_construct!( discovery.poll.register(
      &topic_info_send_timer,
      DISCOVERY_SEND_TOPIC_INFO_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to register topic info sender. {:?}");

    // Participant Message Data 8.4.13
    let participant_message_data_topic = try_construct!( discovery.domain_participant.create_topic(
      "DCPSParticipantMessage",
      "ParticipantMessageData",
      &Discovery::PARTICIPANT_MESSAGE_QOS,
      TopicKind::WithKey,
    ) ,"Unable to create DCPSParticipantMessage topic. {:?}");

    let mut dcps_participant_message_reader = try_construct!( discovery_subscriber
      .create_datareader_CDR::<ParticipantMessageData>(
        participant_message_data_topic.clone(),
        Some(EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER),
        None,
      ) ,"Unable to create DCPSParticipantMessage reader. {:?}");

    try_construct!( discovery.poll.register(
      &dcps_participant_message_reader,
      DISCOVERY_PARTICIPANT_MESSAGE_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to register DCPSParticipantMessage reader. {:?}");

    let mut dcps_participant_message_writer = try_construct!( discovery_publisher
      .create_datawriter_CDR::<ParticipantMessageData>(
        Some(EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER),
        participant_message_data_topic,
        None,
      ) ,"Unable to create DCPSParticipantMessage writer. {:?}");

    let mut dcps_participant_message_timer = mio_extras::timer::Timer::default();
    dcps_participant_message_timer.set_timeout(Discovery::CHECK_PARTICIPANT_MESSAGES, ());
    try_construct!( discovery.poll.register(
      &dcps_participant_message_timer,
      DISCOVERY_PARTICIPANT_MESSAGE_TIMER_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ) ,"Unable to register DCPSParticipantMessage timer. {:?}");

    discovery.initialize_participant(&discovery.domain_participant);

    discovery.write_writers_info(&mut dcps_publication_writer);
    discovery.write_readers_info(&mut dcps_subscription_writer);

    match discovery.discovery_started_sender.send(Ok(())) {
      Ok(_) => (),
      // Participant has probably crashed at this point
      _ => return,
    };

    //println!("Entering discovery event loop");
    loop {
      let mut events = Events::with_capacity(1024);
      match discovery.poll.poll(&mut events, None) {
        Ok(_) => (),
        Err(e) => {
          error!("Failed in waiting of poll in discovery. {:?}", e);
          return;
        }
      }

      for event in events.into_iter() {
        match event.token() {
          DISCOVERY_COMMAND_TOKEN => {
            while let Ok(command) = discovery.discovery_command_receiver.try_recv() {
              match command {
                DiscoveryCommand::STOP_DISCOVERY => {
                  info!("Stopping Discovery");
                  // disposing readers
                  let db = discovery.discovery_db_read();
                  for reader in db.get_all_local_topic_readers() {
                    dcps_subscription_writer
                      .dispose(reader.reader_proxy.remote_reader_guid, None)
                      .unwrap_or(());
                  }

                  for writer in db.get_all_local_topic_writers() {
                    dcps_publication_writer
                      .dispose(writer.writer_proxy.remote_writer_guid, None)
                      .unwrap_or(());
                  }
                  // finally disposing the participant we have
                  dcps_participant_writer.dispose(discovery.domain_participant.get_guid(), None)
                    .unwrap_or(());
                  return  // terminate event loop
                }
                DiscoveryCommand::REMOVE_LOCAL_WRITER { guid } => {
                  if guid == dcps_publication_writer.get_guid() {
                    continue
                  }
                  dcps_publication_writer.dispose(guid, None).unwrap_or(());

                  match discovery.discovery_db.write() {
                    Ok(mut db) => db.remove_local_topic_writer(guid),
                    Err(e) => { error!("DiscoveryDB is poisoned. {:?}", e); return }
                  }
                }
                DiscoveryCommand::REMOVE_LOCAL_READER { guid } => {
                  if guid == dcps_subscription_writer.get_guid() {
                    continue
                  }

                  dcps_subscription_writer.dispose(guid, None).unwrap_or(());

                  match discovery.discovery_db.write() {
                    Ok(mut db) => db.remove_local_topic_reader(guid),
                    Err(e) => { error!("DiscoveryDB is poisoned. {:?}", e); return } 
                  }
                }
                DiscoveryCommand::MANUAL_ASSERT_LIVELINESS => {
                  liveliness_state.last_manual_participant_update = Timestamp::now();
                }
                DiscoveryCommand::ASSERT_TOPIC_LIVELINESS { writer_guid  , manual_assertion } => {
                  discovery.send_discovery_notification(
                    DiscoveryNotificationType::AssertTopicLiveliness { writer_guid , manual_assertion },
                  );
                }
              };
            }
          }

          DISCOVERY_PARTICIPANT_DATA_TOKEN => 
            discovery.handle_participant_reader(&mut dcps_participant_reader),

          DISCOVERY_PARTICIPANT_CLEANUP_TOKEN => {
            discovery.participant_cleanup();
            // setting next cleanup timeout
            participant_cleanup_timer.set_timeout(Discovery::PARTICIPANT_CLEANUP_PERIOD, ());
          }

          DISCOVERY_SEND_PARTICIPANT_INFO_TOKEN => {
            // setting 3 times the duration so lease doesn't break if we fail once for some reason
            let lease_duration = Discovery::SEND_PARTICIPANT_INFO_PERIOD
              + Discovery::SEND_PARTICIPANT_INFO_PERIOD
              + Discovery::SEND_PARTICIPANT_INFO_PERIOD;
            let strong_dp = match discovery.domain_participant.clone().upgrade() {
              Some(dp) => dp,
              None => {
                error!("DomainParticipant doesn't exist anymore, exiting Discovery.");
                return
              }
            };
            let data = SPDPDiscoveredParticipantData::from_participant(
              &strong_dp,
              Duration::from(lease_duration),
            );

            dcps_participant_writer.write(data, None).unwrap_or(());
            // reschedule timer
            participant_send_info_timer.set_timeout(Discovery::SEND_PARTICIPANT_INFO_PERIOD, ());
          }
          DISCOVERY_READER_DATA_TOKEN => {
            discovery.handle_subscription_reader(&mut dcps_subscription_reader);
          }
          DISCOVERY_SEND_READERS_INFO_TOKEN => {
            if discovery.read_readers_info() {
              discovery.write_readers_info(&mut dcps_subscription_writer);
            }

            readers_send_info_timer.set_timeout(Discovery::SEND_READERS_INFO_PERIOD, ());
          }
          DISCOVERY_WRITER_DATA_TOKEN => {
            discovery.handle_publication_reader(&mut dcps_publication_reader);
          }
          DISCOVERY_SEND_WRITERS_INFO_TOKEN => {
            if discovery.read_writers_info() {
              discovery.write_writers_info(&mut dcps_publication_writer);
            }

            writers_send_info_timer.set_timeout(Discovery::SEND_WRITERS_INFO_PERIOD, ());
          }
          DISCOVERY_TOPIC_DATA_TOKEN => {
            discovery.handle_topic_reader(&mut dcps_reader);
          }
          DISCOVERY_TOPIC_CLEANUP_TOKEN => {
            discovery.topic_cleanup();

            topic_cleanup_timer.set_timeout(Discovery::TOPIC_CLEANUP_PERIOD, ());
          }
          DISCOVERY_SEND_TOPIC_INFO_TOKEN => {
            discovery.write_topic_info(&mut dcps_writer);
            topic_info_send_timer.set_timeout(Discovery::SEND_TOPIC_INFO_PERIOD, ());
          }
          DISCOVERY_PARTICIPANT_MESSAGE_TOKEN => {
            discovery.handle_participant_message_reader(&mut dcps_participant_message_reader);
          }
          DISCOVERY_PARTICIPANT_MESSAGE_TIMER_TOKEN => {
            discovery
              .write_participant_message(&mut dcps_participant_message_writer, &mut liveliness_state);
            dcps_participant_message_timer.set_timeout(Discovery::CHECK_PARTICIPANT_MESSAGES, ());
          }
          other_token => {
            error!("discovery event loop got token: {:?}", other_token);
          }
        } // match
      } // for 
    } // loop
  } // fn

  pub fn initialize_participant(&self, dp: &DomainParticipantWeak) {
    let port = get_spdp_well_known_multicast_port(dp.domain_id());
    // TODO: Which Reader? all of them?
    // Or what is the meaning of this? Maybe increase SequenceNumbers to be sent?
    self.send_discovery_notification(
      DiscoveryNotificationType::ParticipantUpdated {
        guid_prefix: dp.get_guid().guidPrefix
    });
    // insert reader proxy as multicast address, so discovery notifications are sent somewhere
    self.initialize_participant_reader_proxy(port);
  }

  pub fn initialize_participant_reader_proxy(&self, port: u16) {
    let guid = GUID::new_with_prefix_and_id(
      GuidPrefix::GUIDPREFIX_UNKNOWN, EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER);

    let mut reader_proxy = ReaderProxy::new(guid);
    reader_proxy.multicast_locator_list = get_local_multicast_locators(port);

    let sub_topic_data = SubscriptionBuiltinTopicData::new(
      guid,
      &String::from("DCPSParticipant"),
      &String::from("SPDPDiscoveredParticipantData"),
      &Discovery::PARTICIPANT_MESSAGE_QOS,
    );
    let drd = DiscoveredReaderData {
      reader_proxy,
      subscription_topic_data: sub_topic_data,
      content_filter: None,
    };

    self.send_discovery_notification(DiscoveryNotificationType::ReaderUpdated
      { rtps_reader_proxy:  RtpsReaderProxy::from_discovered_reader_data(&drd,vec![], vec![]),
        discovered_reader_data: drd,
        needs_new_cache_change: true,
      });
  }


  pub fn handle_participant_reader(&self,
    reader: &mut DataReader<SPDPDiscoveredParticipantData,
      PlCdrDeserializerAdapter<SPDPDiscoveredParticipantData>>) 
  {
    loop {
      let s = reader.take_next_sample();
      match s {
        Ok(Some(d)) => match d.value {
            Ok(participant_data) => {
              debug!("handle_participant_reader discovered {:?}", &participant_data);
              self.discovery_db_write()
                .update_participant(&participant_data);
              self.send_discovery_notification(
                DiscoveryNotificationType::ParticipantUpdated { 
                  guid_prefix: participant_data.participant_guid.guidPrefix 
                } );              
            },
            // Err means that DomainParticipant was disposed
            Err(guid) => {
              self.discovery_db_write().remove_participant(guid.guidPrefix);
              self.send_discovery_notification(
                DiscoveryNotificationType::ParticipantLost { guid_prefix: guid.guidPrefix });
            }
          },
        Ok(None) => return, // no more data
        Err(e) => error!("{:?}",e),
      }
    } // loop
  }

  pub fn handle_subscription_reader(
    &self,
    reader: &mut DataReader<DiscoveredReaderData, PlCdrDeserializerAdapter<DiscoveredReaderData>>,
  ) {
    match reader.take(100, ReadCondition::not_read()) {
      Ok(d) => {
        let mut db = self.discovery_db_write();
        for data in d.into_iter() {
          match data.value() {
            Ok(val) => {
              if let Some( (drd,rtps_reader_proxy) )  = db.update_subscription(&val) {
                self.send_discovery_notification(
                  DiscoveryNotificationType::ReaderUpdated {
                    discovered_reader_data: drd, 
                    rtps_reader_proxy,
                    needs_new_cache_change: true,
                  });  
              }
              db.update_topic_data_drd(&val);
              debug!("Discovered Reader {:?}", &val);
            }
            Err(guid) => {
              debug!("Dispose Reader {:?}", guid);
              db.remove_topic_reader(*guid);
              self.send_discovery_notification(
                  DiscoveryNotificationType::ReaderLost {
                    reader_guid: *guid,
                });
            }
          }
        }
      }
      _ => (),
    };
  }

  pub fn handle_publication_reader(
    &self,
    reader: &mut DataReader<DiscoveredWriterData, PlCdrDeserializerAdapter<DiscoveredWriterData>>,
  ) {
    match reader.take(100, ReadCondition::not_read()) {
      Ok(d) => {
        let mut db = self.discovery_db_write();
        for data in d.into_iter() {
          match data.value() {
            Ok(val) => {
              if let Some(discovered_writer_data) =  db.update_publication(&val) {
                self.send_discovery_notification(
                    DiscoveryNotificationType::WriterUpdated{ discovered_writer_data }
                  );
              }
              db.update_topic_data_dwd(&val);
              debug!("Discovered Writer {:?}", &val);
            }
            Err(writer_guid) => {
              db.remove_topic_writer(*writer_guid);
              self.send_discovery_notification(
                DiscoveryNotificationType::WriterLost { writer_guid: *writer_guid });
              debug!("Disposed Writer {:?}", writer_guid);
            }
          }
        }
      }
      _ => (),
    };
  }

  pub fn handle_topic_reader(
    &self,
    reader: &mut DataReader<DiscoveredTopicData, PlCdrDeserializerAdapter<DiscoveredTopicData>>,
  ) {
    let topic_data_vec: Option<Vec<DiscoveredTopicData>> =
      match reader.take(100, ReadCondition::any()) {
        Ok(d) => Some(
          d.into_iter()
            .map(|p| p.value().clone())
            .filter_map(Result::ok)
            .collect(),
        ),
        _ => None,
      };

    let topic_data_vec = match topic_data_vec {
      Some(d) => d,
      None => return,
    };

    let mut db = self.discovery_db_write();
    topic_data_vec.iter().for_each(|data| {
      let updated = db.update_topic_data(data);
      if updated {
        self.send_discovery_notification(DiscoveryNotificationType::TopicsInfoUpdated);
      }
    });
  }

  // These messages are for updating participant liveliness
  // The protocol distinguises between automatic (by DDS library) 
  // and manual (by by application, via DDS API call) liveness
  pub fn handle_participant_message_reader( &self, reader: &mut DataReader_CDR<ParticipantMessageData> ) {
    let participant_messages: Option<Vec<ParticipantMessageData>> =
      match reader.take(100, ReadCondition::any()) {
        Ok(msgs) => Some(
          msgs
            .into_iter()
            .map(|p| p.value().clone())
            .filter_map(Result::ok)
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
      db.update_lease_duration(msg);
    }
  }

  // TODO: Explain what happens here and by what logic
  pub fn write_participant_message(
    &self,
    writer: &mut DataWriter< ParticipantMessageData, CDRSerializerAdapter<ParticipantMessageData, LittleEndian>,
    >,
    liveliness_state: &mut LivelinessState,
  ) {
    let db = self.discovery_db_read();

    let writer_liveliness: Vec<Liveliness> = db
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
      let current_duration =
        Duration::from(inow.duration_since(liveliness_state.last_auto_update) / 3);
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
        current_duration, min_automatic
      );
      match min_automatic {
        Some(&mm) => {
          if current_duration > mm {
            let pp = ParticipantMessageData {
              guid: self.domain_participant.get_guid_prefix(),
              kind:
                ParticipantMessageDataKind::PARTICIPANT_MESSAGE_DATA_KIND_AUTOMATIC_LIVELINESS_UPDATE,
              data: Vec::new(),
            };
            match writer.write(pp, None) {
              Ok(_) => (),
              Err(e) => {
                error!("Failed to write ParticipantMessageData auto. {:?}", e);
                return;
              }
            }
            liveliness_state.last_auto_update = inow;
          }
        }
        None => (),
      };
    }

    // Manual By Participant
    {
      let current_duration =
        Duration::from(inow.duration_since(liveliness_state.last_manual_participant_update) / 3);
      let min_manual_participant = manual_by_participant
        .iter()
        .map(|lv| match lv {
          Liveliness::Automatic { lease_duration }
          | Liveliness::ManualByParticipant { lease_duration }
          | Liveliness::ManualByTopic { lease_duration } => lease_duration,
        })
        .min();
      match min_manual_participant {
        Some(&dur) => {
          if current_duration > dur {
            let pp = ParticipantMessageData {
              guid: self.domain_participant.get_guid_prefix(),
              kind:
                ParticipantMessageDataKind::PARTICIPANT_MESSAGE_DATA_KIND_MANUAL_LIVELINESS_UPDATE,
              data: Vec::new(),
            };
            match writer.write(pp, None) {
              Ok(_) => (),
              Err(e) => {
                error!("Failed to writer ParticipantMessageData manual. {:?}", e);
                return;
              }
            }
          }
        }
        None => (),
      };
    }
  }

  pub fn participant_cleanup(&self) {
    let removed_guid_prefixes = 
      self.discovery_db_write().participant_cleanup();
    for guid_prefix in removed_guid_prefixes {
      debug!("participant cleanup - timeout for {:?}", guid_prefix);
      self.send_discovery_notification(
                DiscoveryNotificationType::ParticipantLost { guid_prefix });
    }
  }

  pub fn topic_cleanup(&self) {
    self.discovery_db_write().topic_cleanup();
  }

  pub fn read_readers_info(&self) -> bool {
    let readers_info_updated = self.discovery_db_read().is_readers_updated();

    if readers_info_updated {
      self.discovery_db_write().readers_updated(false);
    }

    readers_info_updated
  }

  pub fn read_writers_info(&self) -> bool {
    let writers_info_updated = self.discovery_db_read().is_writers_updated();

    if writers_info_updated {
      self.discovery_db_write().writers_updated(false);
    }

    writers_info_updated
  }

  pub fn write_readers_info(
    &self,
    writer: &mut DataWriter<
      DiscoveredReaderData,
      CDRSerializerAdapter<DiscoveredReaderData, LittleEndian>,
    >,
  ) {
    let db = self.discovery_db_read();
    let datas = db.get_all_local_topic_readers();
    for data in datas
      // filtering out discoveries own readers
      .filter(|p| {
        let eid = p.reader_proxy.remote_reader_guid.entityId;
        eid != EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER
          && eid != EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER
          && eid != EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER
          && eid != EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER
          && eid != EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER
      })
    {
      match writer.write(data.clone(), None) {
        Ok(_) => (),
        Err(e) => error!("Unable to write new readers info. {:?}", e),
      }
    }
  }

  pub fn write_writers_info(
    &self,
    writer: &mut DataWriter<
      DiscoveredWriterData,
      CDRSerializerAdapter<DiscoveredWriterData, LittleEndian>,
    >,
  ) {
    let db = self.discovery_db_read();
    let datas = db.get_all_local_topic_writers();
    for data in datas.filter(|p| {
      let eid = p.writer_proxy.remote_writer_guid.entityId;

      eid != EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER
        && eid != EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER
        && eid != EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER
        && eid != EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER
        && eid != EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER
    }) {
      match writer.write(data.clone(), None) {
        Ok(_) => (),
        _ => error!("Unable to write new readers info."),
      }
    }
  }

  pub fn write_topic_info(
    &self,
    writer: &mut DataWriter<
      DiscoveredTopicData,
      CDRSerializerAdapter<DiscoveredTopicData, LittleEndian>,
    >,
  ) {
    let db = self.discovery_db_read();
    let datas = db.get_all_topics();
    for data in datas {
      match writer.write(data.clone(), None) {
        Ok(_) => (),
        _ => error!("Unable to write new topic info."),
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
      .history(History::KeepLast { depth: 1 }) // there should be no need fo historical data here
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
      .history(History::KeepLast { depth: 1 }) // there should be no need fo historical data here
      // .resource_limits(ResourceLimits { // TODO: Maybe lower limits would suffice?
      //   max_instances: std::i32::MAX,
      //   max_samples: std::i32::MAX,
      //   max_samples_per_instance: std::i32::MAX,
      // })
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
      Err(e) => error!("Failed to send DiscoveryNotification {:?}", e),
    }
  }
}


// -----------------------------------------------------------------------
// -----------------------------------------------------------------------
// -----------------------------------------------------------------------
// -----------------------------------------------------------------------


#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    test::{
      shape_type::ShapeType,
      test_data::{
        spdp_subscription_msg, spdp_publication_msg, spdp_participant_msg_mod,
        create_rtps_data_message,
      },
    },
    network::{udp_listener::UDPListener, udp_sender::UDPSender},
    structure::{entity::RTPSEntity, locator::Locator},
    serialization::{cdr_serializer::to_bytes, cdr_deserializer::CDRDeserializerAdapter},
    messages::submessages::submessages::{InterpreterSubmessage, EntitySubmessage},
    messages::{
      submessages::submessage_elements::serialized_payload::{RepresentationIdentifier},
    },
  };
  use crate::{
    discovery::data_types::topic_data::TopicBuiltinTopicData,
    dds::{participant::DomainParticipant, traits::serde_adapters::DeserializerAdapter},
  };
  use crate::serialization::submessage::*;

  use std::{net::SocketAddr};
  use mio::Token;
  use speedy::{Writable, Endianness};
  use byteorder::LittleEndian;

  #[test]
  fn discovery_participant_data_test() {
    let poll = Poll::new().unwrap();
    let mut udp_listener = UDPListener::new(Token(0), "127.0.0.1", 11000);
    poll
      .register(
        udp_listener.mio_socket(),
        Token(0),
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    // sending participant data to discovery
    let udp_sender = UDPSender::new_with_random_port();
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      get_spdp_well_known_unicast_port(0, 0),
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
    // TODO: we should have received our own participants info decoding the actual message might be good idea
  }

  #[test]
  fn discovery_reader_data_test() {
    let participant = DomainParticipant::new(0).expect("participant creation");

    let topic = participant
      .create_topic(
        "Square",
        "ShapeType",
        &QosPolicies::qos_none(),
        TopicKind::WithKey,
      )
      .unwrap();

    let publisher = participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let _writer = publisher
      .create_datawriter::<ShapeType, CDRSerializerAdapter<ShapeType, LittleEndian>>(
        None, topic.clone(), None,
      )
      .unwrap();

    let subscriber = participant
      .create_subscriber(&QosPolicies::qos_none())
      .unwrap();
    let _reader = subscriber
      .create_datareader::<ShapeType, CDRDeserializerAdapter<ShapeType>>(topic, None, None);

    let poll = Poll::new().unwrap();
    let mut udp_listener = UDPListener::new(Token(0), "127.0.0.1", 11001);
    poll
      .register(
        udp_listener.mio_socket(),
        Token(0),
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    let udp_sender = UDPSender::new_with_random_port();
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      get_spdp_well_known_unicast_port(14, 0),
    )];

    let mut tdata = spdp_subscription_msg();
    let mut data;
    for submsg in tdata.submessages.iter_mut() {
      match &mut submsg.body {
        SubmessageBody::Entity(v) => match v {
          EntitySubmessage::Data(d, _) => {
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

            data = to_bytes::<DiscoveredReaderData, byteorder::LittleEndian>(&drd).unwrap();
            d.serialized_payload.as_mut().unwrap().value = data.clone();
          }
          _ => continue,
        },
        SubmessageBody::Interpreter(_) => (),
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
    let participant = DomainParticipant::new(0);

    let topic = participant
      .create_topic(
        "Square",
        "ShapeType",
        &QosPolicies::qos_none(),
        TopicKind::WithKey,
      )
      .unwrap();

    let publisher = participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let _writer = publisher
      .create_datawriter::<ShapeType, CDRSerializerAdapter<ShapeType, LittleEndian>>(
        None, topic.clone(), None,
      )
      .unwrap();

    let subscriber = participant
      .create_subscriber(&QosPolicies::qos_none())
      .unwrap();
    let _reader = subscriber
      .create_datareader::<ShapeType, CDRDeserializerAdapter<ShapeType>>(topic, None, None);

    let poll = Poll::new().unwrap();
    let mut udp_listener = UDPListener::new(Token(0), "127.0.0.1", 0);
    poll
      .register(
        udp_listener.mio_socket(),
        Token(0),
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    let udp_sender = UDPSender::new_with_random_port();
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      get_spdp_well_known_unicast_port(15, 0),
    )];

    let mut tdata = spdp_publication_msg();
    for submsg in tdata.submessages.iter_mut() {
      match &mut submsg.body {
        SubmessageBody::Interpreter(v) => match v {
          InterpreterSubmessage::InfoDestination(dst, _flags) => {
            dst.guid_prefix = participant.get_guid_prefix().clone();
          }
          _ => continue,
        },
        SubmessageBody::Entity(_) => (),
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

    for _ in udp_listener.get_messages() {
      info!("Message received");
    }
  }

  #[test]
  fn discovery_topic_data_test() {
    let _participant = DomainParticipant::new(0);

    let topic_data = DiscoveredTopicData::new(TopicBuiltinTopicData {
      key: None,
      name: Some(String::from("Square")),
      type_name: Some(String::from("ShapeType")),
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
    });

    let rtps_message = create_rtps_data_message(
      topic_data,
      EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER,
      EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER,
    );

    let udp_sender = UDPSender::new_with_random_port();
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      get_spdp_well_known_unicast_port(16, 0),
    )];

    let rr = rtps_message
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .unwrap();

    udp_sender.send_to_all(&rr, &addresses);
  }
}
