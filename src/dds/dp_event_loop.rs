//use std::cmp::max;

use log::{debug, error, info, warn, trace};
use mio::{Poll, Event, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;
use std::{collections::HashMap, sync::RwLockReadGuard, time::Duration};
use std::{
  sync::{Arc, RwLock},
};

use crate::{
  dds::{message_receiver::MessageReceiver, reader::Reader, writer::Writer, qos::HasQoSPolicy},
  network::util::get_local_multicast_locators,
  structure::builtin_endpoint::BuiltinEndpointSet,
};
use crate::network::udp_listener::UDPListener;
use crate::network::constant::*;
use crate::structure::guid::{GuidPrefix, GUID, EntityId, EntityKind};
use crate::structure::entity::RTPSEntity;
//use crate::structure::sequence_number::SequenceNumber;
use crate::structure::locator::LocatorList;
use crate::{
  common::timed_event_handler::{TimedEventHandler},
  discovery::discovery_db::DiscoveryDB,
  structure::{dds_cache::DDSCache, topic_kind::TopicKind},
  messages::submessages::submessages::AckNack,
};
use crate::dds::with_key::datareader::ReaderCommand;
use super::{
  qos::policy::Reliability, rtps_reader_proxy::RtpsReaderProxy, rtps_writer_proxy::RtpsWriterProxy,
  typedesc::TypeDesc,
};

pub struct DomainInfo {
  pub domain_participant_guid: GUID,
  pub domain_id: u16,
  pub participant_id: u16,
}

pub const PREEMPTIVE_ACKNACK_PERIOD: Duration = Duration::from_secs(5);

// RTPS spec Section 8.4.7.1.1  "Default Timing-Related Values"
pub const NACK_RESPONSE_DELAY: Duration = Duration::from_millis(200); 
pub const NACK_SUPPRESSION_DURATION: Duration = Duration::from_millis(0); 
 
pub struct DPEventLoop {
  domain_info: DomainInfo,
  poll: Poll,
  ddscache: Arc<RwLock<DDSCache>>,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  udp_listeners: HashMap<Token, UDPListener>,
  message_receiver: MessageReceiver, // This contains our Readers

  // Adding readers
  add_reader_receiver: TokenReceiverPair<Reader>,
  remove_reader_receiver: TokenReceiverPair<GUID>,
  reader_timed_event_receiver: HashMap<Token, mio_channel::Receiver<TimerMessageType>>,
  // For each reader a token is added with reades guid it then can be accessed from message receiver
  reader_command_receiver_identification: HashMap<Token, GUID>,

  // Writers
  add_writer_receiver: TokenReceiverPair<Writer>,
  remove_writer_receiver: TokenReceiverPair<GUID>,
  writer_timed_event_reciever: HashMap<Token, mio_channel::Receiver<TimerMessageType>>,

  stop_poll_receiver: mio_channel::Receiver<()>,
  // GuidPrefix sent in this channel needs to be RTPSMessage source_guid_prefix. Writer needs this to locate RTPSReaderProxy if negative acknack.
  ack_nack_reciever: mio_channel::Receiver<(GuidPrefix, AckNack)>,

  writers: HashMap<GUID, Writer>,

  discovery_update_notification_receiver: mio_channel::Receiver<DiscoveryNotificationType>,
}

impl DPEventLoop {
  // This pub(crate) , because it should be constructed only by DomainParticipant.
  pub(crate) fn new(
    domain_info: DomainInfo,
    udp_listeners: HashMap<Token, UDPListener>,
    ddscache: Arc<RwLock<DDSCache>>,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    participant_guid_prefix: GuidPrefix,
    add_reader_receiver: TokenReceiverPair<Reader>,
    remove_reader_receiver: TokenReceiverPair<GUID>,
    add_writer_receiver: TokenReceiverPair<Writer>,
    remove_writer_receiver: TokenReceiverPair<GUID>,
    stop_poll_receiver: mio_channel::Receiver<()>,
    discovery_update_notification_receiver: mio_channel::Receiver<DiscoveryNotificationType>,
  ) -> DPEventLoop {
    let poll = Poll::new().expect("Unable to create new poll.");
    let (acknack_sender, acknack_reciever) =
      mio_channel::sync_channel::<(GuidPrefix, AckNack)>(100);
    let mut udp_listeners = udp_listeners;
    for (token, listener) in &mut udp_listeners {
      poll
        .register(
          listener.mio_socket(),
          token.clone(),
          Ready::readable(),
          PollOpt::edge(),
        )
        .expect("Failed to register listener.");
    }

    poll
      .register(
        &add_reader_receiver.receiver,
        add_reader_receiver.token.clone(),
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register reader adder.");

    poll
      .register(
        &remove_reader_receiver.receiver,
        remove_reader_receiver.token.clone(),
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register reader remover.");
    poll
      .register(
        &add_writer_receiver.receiver,
        add_writer_receiver.token,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register add writer channel");

    poll
      .register(
        &remove_writer_receiver.receiver,
        remove_writer_receiver.token,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register remove writer channel");

    poll
      .register(
        &stop_poll_receiver,
        STOP_POLL_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register stop poll channel");

    poll
      .register(
        &acknack_reciever,
        ACKNACK_MESSGAGE_TO_LOCAL_WRITER_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register AckNack submessage sending from MessageReciever to DPEventLoop");

    poll
      .register(
        &discovery_update_notification_receiver,
        DISCOVERY_UPDATE_NOTIFICATION_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register reader update notification.");

    DPEventLoop {
      domain_info,
      poll,
      ddscache,
      discovery_db,
      udp_listeners,
      message_receiver: MessageReceiver::new(participant_guid_prefix, acknack_sender),
      add_reader_receiver,
      remove_reader_receiver,
      reader_timed_event_receiver: HashMap::new(),
      reader_command_receiver_identification: HashMap::new(),
      add_writer_receiver,
      remove_writer_receiver,
      writer_timed_event_reciever: HashMap::new(),
      stop_poll_receiver,
      writers: HashMap::new(),
      ack_nack_reciever: acknack_reciever,
      discovery_update_notification_receiver,
    }
  }

  pub fn event_loop(self) {
    let mut events = Events::with_capacity(8);  // too small capacity just delays events to next poll
    let mut acknack_timer = mio_extras::timer::Timer::default();
    acknack_timer.set_timeout(PREEMPTIVE_ACKNACK_PERIOD, ());
    self.poll
      .register(&acknack_timer, DPEV_ACKNACK_TIMER_TOKEN, Ready::readable(), PollOpt::edge() )
      .unwrap();

    // TODO: Use the dp to access stuff we need, e.g. historycache
    let mut ev_wrapper = self;
    loop {
      ev_wrapper.poll.poll(&mut events, None)
        .expect("Failed in waiting of poll.");

      for event in events.iter() {
        if event.token() == STOP_POLL_TOKEN {
          info!("Stopping ev_wrapper");
          return
        } else if DPEventLoop::is_udp_traffic(&event) {
          ev_wrapper.handle_udp_traffic(&event);
        } else if DPEventLoop::is_reader_action(&event) {
          ev_wrapper.handle_reader_action(&event);
        } else if ev_wrapper.is_reader_timed_event_action(&event) {
          ev_wrapper.handle_reader_timed_event(&event);
        } else if ev_wrapper.is_reader_command_action(&event) {
          ev_wrapper.handle_reader_command_event(&event);
        } else if DPEventLoop::is_writer_action(&event) {
          ev_wrapper.handle_writer_action(&event);
        } else if ev_wrapper.is_writer_timed_event_action(&event) {
          ev_wrapper.handle_writer_timed_event(&event);
        } else if DPEventLoop::is_writer_acknack_action(&event) {
          ev_wrapper.handle_writer_acknack_action(&event);
        } else if DPEventLoop::is_discovery_update_notification(&event) {
          while let Ok(dnt) = ev_wrapper.discovery_update_notification_receiver.try_recv() {
            match dnt {
              DiscoveryNotificationType::ReadersInfoUpdated => ev_wrapper.update_readers(),
              DiscoveryNotificationType::WritersInfoUpdated { needs_new_cache_change } => 
                ev_wrapper.update_writers(needs_new_cache_change),
              DiscoveryNotificationType::TopicsInfoUpdated => ev_wrapper.update_topics(),
              DiscoveryNotificationType::AssertTopicLiveliness { writer_guid , manual_assertion } => {
                ev_wrapper.writers.get_mut(&writer_guid).map( |w| w.handle_heartbeat_tick(manual_assertion) ); 
              }
            }
          }
        } else if event.token() == DPEV_ACKNACK_TIMER_TOKEN {
          ev_wrapper.message_receiver.send_preemptive_acknacks();
          acknack_timer.set_timeout(PREEMPTIVE_ACKNACK_PERIOD, ());
        } else {
          error!("Unknown event {:?}", event);
        }
      }
    }
  }

  pub fn is_udp_traffic(event: &Event) -> bool {
    event.token() == DISCOVERY_LISTENER_TOKEN
      || event.token() == DISCOVERY_MUL_LISTENER_TOKEN
      || event.token() == USER_TRAFFIC_LISTENER_TOKEN
      || event.token() == USER_TRAFFIC_MUL_LISTENER_TOKEN
  }

  pub fn is_reader_action(event: &Event) -> bool {
    event.token() == ADD_READER_TOKEN || event.token() == REMOVE_READER_TOKEN
  }

  /// Writer action can be add writer remove writer or some not predefined token.
  /// if not predefined token -> EntityIdToken can be calculated and if entityKind is 0xC2 then it is writer action.
  pub fn is_writer_action(event: &Event) -> bool {
    if EntityId::from_usize(event.token().0).is_some() {
      let maybeWriterKind: EntityId = EntityId::from_usize(event.token().0).unwrap();
      if maybeWriterKind.get_kind() == EntityKind::WRITER_WITH_KEY_BUILT_IN
        || maybeWriterKind.get_kind() == EntityKind::WRITER_WITH_KEY_USER_DEFINED
        || maybeWriterKind.get_kind() == EntityKind::WRITER_NO_KEY_BUILT_IN
        || maybeWriterKind.get_kind() == EntityKind::WRITER_NO_KEY_USER_DEFINED
      {
        return true;
      }
    }
    event.token() == ADD_WRITER_TOKEN || event.token() == REMOVE_WRITER_TOKEN
  }

  /// Writer timed events can be Heartbeats or cache cleaning actions.
  pub fn is_writer_timed_event_action(&self, event: &Event) -> bool {
    self.writer_timed_event_reciever.contains_key(&event.token())
  }

  pub fn is_reader_timed_event_action(&self, event: &Event) -> bool {
    self.reader_timed_event_receiver.contains_key(&event.token())
  }

  pub fn is_reader_command_action(&self, event: &Event) -> bool {
    self
      .reader_command_receiver_identification
      .contains_key(&event.token())
  }

  pub fn is_writer_acknack_action(event: &Event) -> bool {
    event.token() == ACKNACK_MESSGAGE_TO_LOCAL_WRITER_TOKEN
  }

  pub fn is_discovery_update_notification(event: &Event) -> bool {
    event.token() == DISCOVERY_UPDATE_NOTIFICATION_TOKEN
  }

  pub fn handle_udp_traffic(&mut self, event: &Event) {
    let listener = self.udp_listeners.get(&event.token());
    let datas;
    match listener {
      Some(l) => datas = l.get_messages(),
      None => {
        error!("handle_udp_traffic - internal error! No listener with token {:?}", &event.token() );
        return
      }
    };
    for data in datas.into_iter() {
      if event.token() == DISCOVERY_LISTENER_TOKEN || event.token() == DISCOVERY_MUL_LISTENER_TOKEN
      {
        self.message_receiver.handle_discovery_msg(data);
      } else if event.token() == USER_TRAFFIC_LISTENER_TOKEN
        || event.token() == USER_TRAFFIC_MUL_LISTENER_TOKEN
      {
        self.message_receiver.handle_user_msg(data);
      }
    }
  }

  pub fn handle_reader_action(&mut self, event: &Event) {
    match event.token() {
      ADD_READER_TOKEN => {
        trace!("add reader(s)");
        while let Ok(mut new_reader) = self.add_reader_receiver.receiver.try_recv() {
          let (timed_action_sender, timed_action_receiver) =
            mio_channel::sync_channel::<TimerMessageType>(10);
          let time_handler: TimedEventHandler = TimedEventHandler::new(timed_action_sender.clone());
          new_reader.add_timed_event_handler(time_handler);

          self
            .poll
            .register(
              &timed_action_receiver,
              new_reader.get_entity_token(),
              Ready::readable(),
              PollOpt::edge(),
            )
            .expect("Reader timer channel registeration failed!");
          self
            .reader_timed_event_receiver
            .insert(new_reader.get_entity_token(), timed_action_receiver);

          self
            .poll
            .register(
              &new_reader.data_reader_command_receiver,
              new_reader.get_reader_command_entity_token(),
              Ready::readable(),
              PollOpt::edge(),
            )
            .expect("Reader command channel registration failed!!!");

          self.reader_command_receiver_identification.insert(
            new_reader.get_reader_command_entity_token(),
            new_reader.get_guid(),
          );
          new_reader.set_requested_deadline_check_timer();
          trace!("Add reader: {:?}", new_reader);
          self.message_receiver.add_reader(new_reader);
        }
      }
      REMOVE_READER_TOKEN => {
        while let Ok(old_reader_guid) = self.remove_reader_receiver.receiver.try_recv() {
          self.message_receiver.remove_reader(old_reader_guid);
        }
      }
      _ => {}
    }
  }

  pub fn handle_writer_action(&mut self, event: &Event) {
    match event.token() {
      ADD_WRITER_TOKEN => {
        while let Ok(mut new_writer) = self.add_writer_receiver.receiver.try_recv() {
          &self.poll.register(
            &new_writer.writer_command_receiver,
            new_writer.get_entity_token(),
            Ready::readable(),
            PollOpt::edge(),
          );
          let (timed_action_sender, timed_action_receiver) =
            mio_channel::sync_channel::<TimerMessageType>(10);
          let time_handler: TimedEventHandler = TimedEventHandler::new(timed_action_sender.clone());
          new_writer.add_timed_event_handler(time_handler);

          self.poll.register(
              &timed_action_receiver,
              new_writer.get_timed_event_entity_token(),
              Ready::readable(),
              PollOpt::edge(),
            )
            .expect("Writer heartbeat timer channel registration failed!!");
          self.writer_timed_event_reciever.insert(
            new_writer.get_timed_event_entity_token(),
            timed_action_receiver,
          );
          self.writers.insert(new_writer.get_guid(), new_writer);
        }
      }
      REMOVE_WRITER_TOKEN => {
        while let Ok(writer_guid) = &self.remove_writer_receiver.receiver.try_recv() {
          let writer = self.writers.remove(writer_guid);
          if let Some(w) = writer {
            &self.poll.deregister(&w.writer_command_receiver);
          };
        }
      }

      writer_token => {
        self
          .writers
          .iter_mut()
          .find(|p| p.1.get_entity_token() == writer_token)
          .map( |(_guid, writer)| writer.process_writer_command() );
      }
    }
  }

  /// Writer timed events can be heatrbeats or cache cleaning events.
  /// events are distinguished by TimerMessageType which is send via mio channel. Channel token in
  pub fn handle_writer_timed_event(&mut self, event: &Event) {
    let reciever = self.writer_timed_event_reciever.get(&event.token())
      .expect("Did not find a heartbeat receiver");
    while let Ok(timer_message) = reciever.try_recv() {
      self.writers.iter_mut()
          .find(|(_guid,writer)| writer.get_timed_event_entity_token() == event.token())
          .map( |(_guid, writer)| writer.handle_timed_event(timer_message));
    }
  }

  pub fn handle_reader_timed_event(&mut self, event: &Event) {
    let reciever = self.reader_timed_event_receiver.get(&event.token())
      .expect("Did not found reader timed event reciever!");
    // TODO: Why do we have separate message channels for reader and writer, if they have contain the same data type?

    while let Ok(timer_message) = reciever.try_recv() {
      match self.message_receiver.available_readers.iter_mut()
          .find(|reader| reader.get_entity_token() == event.token()) {
        Some(reader) => {
          reader.handle_timed_event(timer_message);
        }
        None => error!("Reader was not found with entity token {:?}",  event.token()),
      }
    }
  }

  pub fn handle_reader_command_event(&mut self, event: &Event) {
    let reader_guid = self
      .reader_command_receiver_identification
      .get(&event.token())
      .expect(&"Did not found reader command reciever!");
    let reader = self
      .message_receiver
      .get_reader(reader_guid.entityId)
      .expect(&format!(
        "Message received id not contain reade with entityID: {:?}",
        reader_guid.entityId
      ));

    let mut message_queue: Vec<ReaderCommand> = vec![];
    while let Ok(res) = reader.data_reader_command_receiver.try_recv() {
      message_queue.push(res);
    }

    for command in message_queue {
      match command {
        ReaderCommand::RESET_REQUESTED_DEADLINE_STATUS => {
          reader.reset_requested_deadline_missed_status();
        }
      }
    }
  }

  pub fn handle_writer_acknack_action(&mut self, _event: &Event) {
    while let Ok((acknack_sender_prefix, acknack_message)) = self.ack_nack_reciever.try_recv() {
      let target_writer_entity_id = { acknack_message.writer_id };
      let writer_guid = GUID::new_with_prefix_and_id(
        self.domain_info.domain_participant_guid.guidPrefix,
        target_writer_entity_id,
      );
      if let Some(found_writer) = self.writers.get_mut(&writer_guid) {
        if found_writer.is_reliable() {
          found_writer.handle_ack_nack(acknack_sender_prefix, acknack_message)
        }
      } else {
        warn!(
          "Couldn't handle acknack! did not find local rtps writer with GUID: {:x?}",
          writer_guid
        );
        continue;
      }
    }
  }

  pub fn update_writers(&mut self, needs_new_cache_change: bool) {

    match self.discovery_db.read() {
      Ok(db) => {
        for (_writer_guid, writer) in self.writers.iter_mut() {
          if writer.get_entity_id() == EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER {
            DPEventLoop::update_spdp_participant_readers(
              writer,
              &db,
              self.domain_info.domain_id,
            );

            if needs_new_cache_change {
              writer.notify_new_data_to_all_readers()
            }
          } else if writer.get_entity_id() == EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER {
            DPEventLoop::update_pubsub_readers(
              writer,
              &db,
              EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER,
              BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR,
            );
            if needs_new_cache_change {
              writer.notify_new_data_to_all_readers()
            }
          } else if writer.get_entity_id() == EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER {
            DPEventLoop::update_pubsub_readers(
              writer,
              &db,
              EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
              BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR,
            );
            if needs_new_cache_change {
              writer.notify_new_data_to_all_readers()
            }
          } else if writer.get_entity_id() == EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER {
            // TODO:
          } else if writer.get_entity_id()
            == EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER
          {
            DPEventLoop::update_pubsub_readers(
              writer,
              &db,
              EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER,
              BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_MESSAGE_DATA_READER,
            );
            if needs_new_cache_change {
              writer.notify_new_data_to_all_readers()
            }
          } else {
            writer.readers = db
              .get_external_reader_proxies()
              .filter(|p| match p.subscription_topic_data.topic_name().as_ref() {
                Some(tn) => *writer.topic_name() == *tn,
                None => false,
              })
              .map(|drd| {
                  // find out default LocatorsLists from Participant proxy
                  let remote_reader_guid = drd.reader_proxy.remote_reader_guid;
                  let locator_lists = 
                    db.find_participant_proxy(remote_reader_guid.guidPrefix)
                      .map(|pp| {
                            debug!("Added default locators to Reader {:?}", remote_reader_guid);
                            ( pp.default_unicast_locators.clone(), 
                              pp.default_multicast_locators.clone() ) } )
                      .unwrap_or_else( || {
                          warn!("No remote participant known for {:?}",drd);
                          (LocatorList::new(), LocatorList::new()) 
                        } );
                  // create new reader proxy
                  RtpsReaderProxy::from_discovered_reader_data(drd, 
                    locator_lists.0 , locator_lists.1)
                }
              )
              .collect();

            if let Some(Reliability::Reliable { max_blocking_time: _, }) 
                  = writer.get_qos().reliability
            {
              // reset data sending
              writer.notify_new_data_to_all_readers()
            }
          }
        }
      }
      _ => panic!("DiscoveryDB is poisoned."),
    }
  }

  fn update_spdp_participant_readers(
    writer: &mut Writer,
    db: &RwLockReadGuard<DiscoveryDB>,
    domain_id: u16,
  ) {
    let guid_prefix = writer.get_guid_prefix();

    // generating readers from all found participants
    let all_readers = db
      .get_participants()
      .filter_map(|p| match p.participant_guid {
        Some(g) => Some((g, p)),
        None => None,
      })
      .filter(|(_, sp)| match sp.available_builtin_endpoints {
        Some(ep) => ep.contains(BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR),
        None => false,
      })
      .filter(|(g, _)| g.guidPrefix != guid_prefix)
      .map(|(g, p)| {
        (
          g,
          p.as_reader_proxy(
            true,
            Some(EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER),
          ),
        )
      });

    // updating all data
    for reader in all_readers.map(|(_, p)| p).into_iter() {
      DPEventLoop::add_reader_to_writer(writer, reader);
    }

    // adding multicast reader
    let multicast_guid = GUID::new_with_prefix_and_id(
      GuidPrefix::GUIDPREFIX_UNKNOWN,
      EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
    );

    let mut multicast_reader = RtpsReaderProxy::new(multicast_guid);
    multicast_reader.multicast_locator_list =
      get_local_multicast_locators(get_spdp_well_known_multicast_port(domain_id));

    DPEventLoop::add_reader_to_writer(writer, multicast_reader);
    debug!("SPDP Participant readers updated.");
  }

  fn update_pubsub_readers(
    writer: &mut Writer,
    db: &RwLockReadGuard<DiscoveryDB>,
    entity_id: EntityId,
    expected_endpoint: u32,
  ) {
    let guid_prefix = writer.get_guid_prefix();
    // getting all possible readers
    let all_readers = db
      .get_participants()
      .filter_map(|p| match p.participant_guid {
        Some(g) => Some((g, p)),
        None => None,
      })
      .filter(|(_, sp)| match sp.available_builtin_endpoints {
        Some(ep) => ep.contains(expected_endpoint),
        None => false,
      })
      .filter(|(g, _)| g.guidPrefix != guid_prefix)
      .map(|(g, p)| (g, p.as_reader_proxy(true, Some(entity_id))));

    // updating all data
    for reader in all_readers.map(|(_, p)| p).into_iter() {
      DPEventLoop::add_reader_to_writer(writer, reader);
    }
  }

  fn update_pubsub_writers(
    reader: &mut Reader,
    db: &RwLockReadGuard<DiscoveryDB>,
    entity_id: EntityId,
    expected_endpoint: u32,
  ) {
    let guid_prefix = reader.get_guid_prefix();

    let all_writers = db
      .get_participants()
      .filter_map(|p| match p.participant_guid {
        Some(g) => Some((g, p)),
        None => None,
      })
      .filter(|(_, sp)| match sp.available_builtin_endpoints {
        Some(ep) => ep.contains(expected_endpoint),
        None => false,
      })
      .filter(|(g, _)| g.guidPrefix != guid_prefix)
      .map(|(g, p)| (g, p.as_writer_proxy(true, Some(entity_id))));

    for writer in all_writers.map(|(_, p)| p).into_iter() {
      reader.add_writer_proxy(writer);
    }
  }

  fn add_reader_to_writer(writer: &mut Writer, proxy: RtpsReaderProxy) {
    let reader = writer.readers
                  .iter_mut()
                  .find(|r| r.remote_reader_guid == proxy.remote_reader_guid);


    // TODO: Isn't this bad? It could cause ghost seuquence numbers, if
    // nothing is sent yet.
    /*proxy.notify_new_cache_change(
      max(writer.last_change_sequence_number, SequenceNumber::default() )
    );*/

    // TODO: We should initialize reader proxy so that it does not try to send
    // all data from beginning of time, but sends a reasonable GAP message first.

    match reader {
      Some(r) => {
        if !r.content_is_equal(&proxy) {
          *r = proxy
        }
      }
      None => writer.readers.push(proxy), // TODO: This should be a method call
    };
  }

  pub fn update_readers(&mut self) {
    let db = match self.discovery_db.read() {
      Ok(db) => db,
      Err(e) => panic!("DiscoveryDB is poisoned. {:?}", e),
    };

    for reader in self.message_receiver.available_readers.iter_mut() {
      match reader.get_entity_id() {
        EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER => {
          let proxies: Vec<RtpsWriterProxy> = db
            .get_participants()
            .filter(|sp| match sp.available_builtin_endpoints {
              Some(ep) => {
                ep.contains(BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PARTICIPANT_ANNOUNCER)
              }
              None => false,
            })
            .filter(|p| p.participant_guid.unwrap().guidPrefix != reader.get_guid_prefix())
            .map(|p| {
              p.as_writer_proxy(
                true,
                Some(EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER),
              )
            })
            .collect();

          reader.retain_matched_writers(proxies.iter());
          for proxy in proxies.into_iter() {
            reader.add_writer_proxy(proxy);
          }
        }
        EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER => {
          DPEventLoop::update_pubsub_writers(
            reader,
            &db,
            EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER,
          );
        }
        EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER => {
          DPEventLoop::update_pubsub_writers(
            reader,
            &db,
            EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER,
          );
        }
        EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER => {
          DPEventLoop::update_pubsub_writers(
            reader,
            &db,
            EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
            BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_MESSAGE_DATA_WRITER,
          );
        }
        _ => {
          let topic_name = reader.topic_name().clone();
          let proxies: Vec<RtpsWriterProxy> = db
            .get_external_writer_proxies()
            .filter(|p| match p.publication_topic_data.topic_name.as_ref() {
              Some(tn) => topic_name == *tn,
              None => false,
            })
            .map(|p| RtpsWriterProxy::from_discovered_writer_data(p))
            .collect();

          reader.retain_matched_writers(proxies.iter());
          for proxy in proxies.into_iter() {
            reader.add_writer_proxy(proxy);
          }
        }
      }
    }
  }

  pub fn update_topics(&mut self) {
    match self.discovery_db.read() {
      Ok(db) => match self.ddscache.write() {
        Ok(mut ddsc) => {
          for topic in db.get_all_topics() {
            let topic_name = match &topic.topic_data.name {
              Some(td) => td,
              None => continue,
            };
            // TODO: how do you know when topic is keyed and is not
            let topic_kind = match &topic.topic_data.key {
              Some(_) => TopicKind::WithKey,
              None => TopicKind::NoKey,
            };
            let topic_data_type = match &topic.topic_data.type_name {
              Some(tn) => tn.clone(),
              None => continue,
            };
            ddsc.add_new_topic(topic_name, topic_kind, TypeDesc::new(topic_data_type));
          }
        }
        _ => panic!("DDSCache is poisoned"),
      },
      _ => panic!("DiscoveryDB is poisoned"),
    }
  }
}

// -----------------------------------------------------------
// -----------------------------------------------------------
// -----------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use std::thread;
  use std::time::Duration;
  use crate::{
    dds::participant::DomainParticipant,
    dds::values::result::CountWithChange,
    dds::values::result::RequestedDeadlineMissedStatus,
    dds::with_key::datareader::DataReader,
    serialization::cdr_deserializer::CDRDeserializerAdapter,
    structure::duration::Duration as DurationDDS,
    test::{random_data::RandomData, datareader_util::DataReaderTestUtil},
  };
  use mio::{Ready, PollOpt};
  use crate::{
    dds::with_key::datareader::ReaderCommand,
    dds::{qos::policy::Deadline, values::result::StatusChange},
    structure::entity::RTPSEntity,
    dds::qos::QosPolicies,
  };
  use crate::structure::dds_cache::DDSCache;
  use crate::dds::topic::Topic;

  #[test]
  fn dpew_add_and_remove_readers() {
    // Adding readers
    let (sender_add_reader, receiver_add) = mio_channel::channel::<Reader>();
    let (sender_remove_reader, receiver_remove) = mio_channel::channel::<GUID>();

    let (_add_writer_sender, add_writer_receiver) = mio_channel::channel();
    let (_remove_writer_sender, remove_writer_receiver) = mio_channel::channel();

    let (_stop_poll_sender, stop_poll_receiver) = mio_channel::channel();

    let (_discovery_update_notification_sender, discovery_update_notification_receiver) =
      mio_channel::channel();

    let ddshc = Arc::new(RwLock::new(DDSCache::new()));
    let discovery_db = Arc::new(RwLock::new(DiscoveryDB::new()));

    let domain_info = DomainInfo {
      domain_participant_guid: GUID::new(),
      domain_id: 0,
      participant_id: 0,
    };

    let dp_event_loop = DPEventLoop::new(
      domain_info,
      HashMap::new(),
      ddshc,
      discovery_db,
      GuidPrefix::default(),
      TokenReceiverPair {
        token: ADD_READER_TOKEN,
        receiver: receiver_add,
      },
      TokenReceiverPair {
        token: REMOVE_READER_TOKEN,
        receiver: receiver_remove,
      },
      TokenReceiverPair {
        token: ADD_WRITER_TOKEN,
        receiver: add_writer_receiver,
      },
      TokenReceiverPair {
        token: REMOVE_WRITER_TOKEN,
        receiver: remove_writer_receiver,
      },
      stop_poll_receiver,
      discovery_update_notification_receiver,
    );

    let (sender_stop, receiver_stop) = mio_channel::channel::<i32>();
    dp_event_loop
      .poll
      .register(
        &receiver_stop,
        STOP_POLL_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register receivers.");

    let child = thread::spawn(move || DPEventLoop::event_loop(dp_event_loop));

    let n = 3;

    let mut reader_guids = Vec::new();
    for i in 0..n {
      let new_guid = GUID::new();

      let (send, _rec) = mio_channel::sync_channel::<()>(100);
      let (status_sender, _status_reciever) =
        mio_extras::channel::sync_channel::<StatusChange>(100);
      let (_reader_commander, reader_command_receiver) =
        mio_extras::channel::sync_channel::<ReaderCommand>(100);

      let new_reader = Reader::new(
        new_guid,
        send,
        status_sender,
        Arc::new(RwLock::new(DDSCache::new())),
        "test".to_string(),
        reader_command_receiver,
      );

      reader_guids.push(new_reader.get_guid().clone());
      info!("\nSent reader number {}: {:?}\n", i, &new_reader);
      sender_add_reader.send(new_reader).unwrap();
      std::thread::sleep(Duration::new(0, 100));
    }

    info!("\npoistetaan toka\n");
    let some_guid = reader_guids[1].clone();
    sender_remove_reader.send(some_guid).unwrap();
    std::thread::sleep(Duration::new(0, 100));

    info!("\nLopetustoken lähtee\n");
    sender_stop.send(0).unwrap();
    child.join().unwrap();
  }

  #[test]
  fn dpew_test_reader_commands() {
    let somePolicies = QosPolicies {
      durability: None,
      presentation: None,
      deadline: Some(Deadline(DurationDDS::from_millis(500))),
      latency_budget: None,
      ownership: None,
      liveliness: None,
      time_based_filter: None,
      reliability: None,
      destination_order: None,
      history: None,
      resource_limits: None,
      lifespan: None,
    };
    let dp = DomainParticipant::new(0);
    let sub = dp.create_subscriber(&somePolicies).unwrap();

    let topic_1 = dp
      .create_topic("TOPIC_1", "jotain", &somePolicies, TopicKind::WithKey)
      .unwrap();
    let _topic_2 = dp
      .create_topic("TOPIC_2", "jotain", &somePolicies, TopicKind::WithKey)
      .unwrap();
    let _topic_3 = dp
      .create_topic("TOPIC_3", "jotain", &somePolicies, TopicKind::WithKey)
      .unwrap();

    // Adding readers
    let (sender_add_reader, receiver_add) = mio_channel::channel::<Reader>();
    let (_sender_remove_reader, receiver_remove) = mio_channel::channel::<GUID>();

    let (_add_writer_sender, add_writer_receiver) = mio_channel::channel();
    let (_remove_writer_sender, remove_writer_receiver) = mio_channel::channel();

    let (_stop_poll_sender, stop_poll_receiver) = mio_channel::channel();

    let (_discovery_update_notification_sender, discovery_update_notification_receiver) =
      mio_channel::channel();

    let ddshc = Arc::new(RwLock::new(DDSCache::new()));
    let discovery_db = Arc::new(RwLock::new(DiscoveryDB::new()));

    let domain_info = DomainInfo {
      domain_participant_guid: GUID::new(),
      domain_id: 0,
      participant_id: 0,
    };

    let dp_event_loop = DPEventLoop::new(
      domain_info,
      HashMap::new(),
      ddshc,
      discovery_db,
      GuidPrefix::default(),
      TokenReceiverPair {
        token: ADD_READER_TOKEN,
        receiver: receiver_add,
      },
      TokenReceiverPair {
        token: REMOVE_READER_TOKEN,
        receiver: receiver_remove,
      },
      TokenReceiverPair {
        token: ADD_WRITER_TOKEN,
        receiver: add_writer_receiver,
      },
      TokenReceiverPair {
        token: REMOVE_WRITER_TOKEN,
        receiver: remove_writer_receiver,
      },
      stop_poll_receiver,
      discovery_update_notification_receiver,
    );

    let (sender_stop, receiver_stop) = mio_channel::channel::<i32>();
    dp_event_loop
      .poll
      .register(
        &receiver_stop,
        STOP_POLL_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register receivers.");

    let child = thread::spawn(move || DPEventLoop::event_loop(dp_event_loop));

    //TODO IF THIS IS SET TO 1 TEST SUCCEEDS
    let n = 1;

    let mut reader_guids = Vec::new();
    let mut data_readers: Vec<DataReader<RandomData, CDRDeserializerAdapter<RandomData>>> = vec![];
    let _topics: Vec<Topic> = vec![];
    for i in 0..n {
      //topics.push(topic);
      let new_guid = GUID::new();

      let (send, _rec) = mio_channel::sync_channel::<()>(100);
      let (status_sender, status_reciever_DataReader) =
        mio_extras::channel::sync_channel::<StatusChange>(1000);
      let (reader_commander, reader_command_receiver) =
        mio_extras::channel::sync_channel::<ReaderCommand>(1000);

      let mut new_reader = Reader::new(
        new_guid,
        send,
        status_sender,
        Arc::new(RwLock::new(DDSCache::new())),
        "test".to_string(),
        reader_command_receiver,
      );

      let somePolicies = QosPolicies {
        durability: None,
        presentation: None,
        deadline: Some(Deadline(DurationDDS::from_millis(50))),
        latency_budget: None,
        ownership: None,
        liveliness: None,
        time_based_filter: None,
        reliability: None,
        destination_order: None,
        history: None,
        resource_limits: None,
        lifespan: None,
      };

      let mut datareader = sub
        .create_datareader::<RandomData, CDRDeserializerAdapter<RandomData>>(
          topic_1.clone(),
          Some(EntityId::ENTITYID_UNKNOWN),
          Some(somePolicies.clone()),
        )
        .unwrap();

      datareader.set_status_change_receiver(status_reciever_DataReader);
      datareader.set_reader_commander(reader_commander);
      data_readers.push(datareader);

      //new_reader.set_qos(&somePolicies).unwrap();
      new_reader.matched_writer_add(GUID::new(), EntityId::ENTITYID_UNKNOWN, vec![], vec![]);
      reader_guids.push(new_reader.get_guid().clone());
      info!("\nSent reader number {}: {:?}\n", i, &new_reader);
      sender_add_reader.send(new_reader).unwrap();
      std::thread::sleep(Duration::from_millis(100));
    }
    thread::sleep(Duration::from_millis(100));

    let status = data_readers
      .get_mut(0)
      .unwrap()
      .get_requested_deadline_missed_status();
    info!("Received status change: {:?}", status);
    assert_eq!(
      status.unwrap(),
      Some(RequestedDeadlineMissedStatus::from_count(
        CountWithChange::start_from(3, 3)
      )),
    );
    thread::sleep(Duration::from_millis(150));

    let status2 = data_readers
      .get_mut(0)
      .unwrap()
      .get_requested_deadline_missed_status();
    info!("Received status change: {:?}", status2);
    assert_eq!(
      status2.unwrap(),
      Some(RequestedDeadlineMissedStatus::from_count(
        CountWithChange::start_from(6, 3)
      ))
    );

    let status3 = data_readers
      .get_mut(0)
      .unwrap()
      .get_requested_deadline_missed_status();
    info!("Received status change: {:?}", status3);
    assert_eq!(
      status3.unwrap(),
      Some(RequestedDeadlineMissedStatus::from_count(
        CountWithChange::start_from(6, 0)
      ))
    );

    thread::sleep(Duration::from_millis(50));

    let status4 = data_readers
      .get_mut(0)
      .unwrap()
      .get_requested_deadline_missed_status();
    info!("Received status change: {:?}", status4);
    assert_eq!(
      status4.unwrap(),
      Some(RequestedDeadlineMissedStatus::from_count(
        CountWithChange::start_from(7, 1)
      ))
    );

    info!("\nLopetustoken lähtee\n");
    sender_stop.send(0).unwrap();
    child.join().unwrap();
  }
}
