use log::{debug, info, warn};
use mio::{Poll, Event, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;
extern crate chrono;
//use chrono::Duration;
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
use crate::structure::guid::{GuidPrefix, GUID, EntityId};
use crate::structure::entity::Entity;
use crate::{
  common::timed_event_handler::{TimedEventHandler},
  discovery::discovery_db::DiscoveryDB,
  structure::{dds_cache::DDSCache, topic_kind::TopicKind},
  submessages::AckNack,
};
use super::{
  typedesc::TypeDesc, rtps_reader_proxy::RtpsReaderProxy, rtps_writer_proxy::RtpsWriterProxy,
  qos::policy::Reliability,
};

pub struct DomainInfo {
  pub domain_participant_guid: GUID,
  pub domain_id: u16,
  pub participant_id: u16,
}

pub struct DPEventWrapper {
  domain_info: DomainInfo,
  poll: Poll,
  ddscache: Arc<RwLock<DDSCache>>,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  udp_listeners: HashMap<Token, UDPListener>,
  message_receiver: MessageReceiver,

  // Adding readers
  add_reader_receiver: TokenReceiverPair<Reader>,
  remove_reader_receiver: TokenReceiverPair<GUID>,

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

impl DPEventWrapper {
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
  ) -> DPEventWrapper {
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

    DPEventWrapper {
      domain_info,
      poll,
      ddscache,
      discovery_db,
      udp_listeners,
      message_receiver: MessageReceiver::new(participant_guid_prefix, acknack_sender),
      add_reader_receiver,
      remove_reader_receiver,
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
    let mut acknack_timer = mio_extras::timer::Timer::default();
    acknack_timer.set_timeout(Duration::from_secs(5), ());
    self
      .poll
      .register(
        &acknack_timer,
        DPEV_ACKNACK_TIMER_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    // TODO: Use the dp to access stuff we need, e.g. historycache
    let mut ev_wrapper = self;
    loop {
      let mut events = Events::with_capacity(1024);
      ev_wrapper
        .poll
        .poll(&mut events, None)
        .expect("Failed in waiting of poll.");

      for event in events.into_iter() {
        if event.token() == STOP_POLL_TOKEN {
          info!("Stopping ev_wrapper");
          return;
        } else if DPEventWrapper::is_udp_traffic(&event) {
          ev_wrapper.handle_udp_traffic(&event);
        } else if DPEventWrapper::is_reader_action(&event) {
          ev_wrapper.handle_reader_action(&event);
        } else if DPEventWrapper::is_writer_action(&event) {
          ev_wrapper.handle_writer_action(&event);
        } else if ev_wrapper.is_writer_timed_event_action(&event) {
          ev_wrapper.handle_writer_timed_event(&event);
        } else if DPEventWrapper::is_writer_acknack_action(&event) {
          ev_wrapper.handle_writer_acknack_action(&event);
        } else if DPEventWrapper::is_discovery_update_notification(&event) {
          while let Ok(dnt) = ev_wrapper.discovery_update_notification_receiver.try_recv() {
            match dnt {
              DiscoveryNotificationType::ReadersInfoUpdated => ev_wrapper.update_readers(),
              DiscoveryNotificationType::WritersInfoUpdated {
                needs_new_cache_change,
              } => ev_wrapper.update_writers(needs_new_cache_change),
              DiscoveryNotificationType::TopicsInfoUpdated => ev_wrapper.update_topics(),
            }
          }
        } else if event.token() == DPEV_ACKNACK_TIMER_TOKEN {
          ev_wrapper.message_receiver.send_preemptive_acknacks();
          acknack_timer.set_timeout(Duration::from_secs(5), ());
        } else {
          info!("Unknown event");
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
      if maybeWriterKind.get_kind() == 0xC2
        || maybeWriterKind.get_kind() == 0x02
        || maybeWriterKind.get_kind() == 0xC3
        || maybeWriterKind.get_kind() == 0x03
      {
        return true;
      }
    }
    event.token() == ADD_WRITER_TOKEN || event.token() == REMOVE_WRITER_TOKEN
  }

  /// Writer timed events can be Heartbeats or cache cleaning actions.
  pub fn is_writer_timed_event_action(&self, event: &Event) -> bool {
    if self
      .writer_timed_event_reciever
      .contains_key(&event.token())
    {
      return true;
    }
    return false;
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
        print!(
          "Cannot handle upd traffic! No listener with token {:?}",
          &event.token()
        );
        return;
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
        while let Ok(new_reader) = self.add_reader_receiver.receiver.try_recv() {
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
            new_writer.cache_change_receiver(),
            new_writer.get_entity_token(),
            Ready::readable(),
            PollOpt::edge(),
          );
          let (timed_action_sender, timed_action_receiver) =
            mio_channel::sync_channel::<TimerMessageType>(10);
          let time_handler: TimedEventHandler = TimedEventHandler::new(timed_action_sender.clone());
          new_writer.add_timed_event_handler(time_handler);

          self
            .poll
            .register(
              &timed_action_receiver,
              new_writer.get_timed_event_entity_token(),
              Ready::readable(),
              PollOpt::edge(),
            )
            .expect("Writer heartbeat timer channel registeration failed!!");
          self.writer_timed_event_reciever.insert(
            new_writer.get_timed_event_entity_token(),
            timed_action_receiver,
          );
          self.writers.insert(new_writer.as_entity().guid, new_writer);
        }
      }
      REMOVE_WRITER_TOKEN => {
        while let Ok(writer_guid) = &self.remove_writer_receiver.receiver.try_recv() {
          let writer = self.writers.remove(writer_guid);
          if let Some(w) = writer {
            &self.poll.deregister(w.cache_change_receiver());
          };
        }
      }
      t => {
        let found_writer = self
          .writers
          .iter_mut()
          .find(|p| p.1.get_entity_token() == t);

        match found_writer {
          Some((_guid, w)) => {
            while let Ok(cc) = w.cache_change_receiver().try_recv() {
              w.insert_to_history_cache(cc);
              w.send_all_unsend_messages();
            }
          }
          None => {}
        }
      }
    }
  }

  /// Writer timed events can be heatrbeats or cache cleaning events.
  /// events are distinguished by TimerMessageType which is send via mio channel. Channel token in
  pub fn handle_writer_timed_event(&mut self, event: &Event) {
    let reciever = self
      .writer_timed_event_reciever
      .get(&event.token())
      .expect("Did not found heartbeat reciever ");
    let mut message_queue: Vec<TimerMessageType> = vec![];
    while let Ok(res) = reciever.try_recv() {
      message_queue.push(res);
    }

    for timer_message in message_queue {
      if timer_message == TimerMessageType::writer_heartbeat {
        let found_writer_with_heartbeat = self
          .writers
          .iter_mut()
          .find(|p| p.1.get_timed_event_entity_token() == event.token());
        match found_writer_with_heartbeat {
          Some((_guid, w)) => {
            w.handle_heartbeat_tick();
          }
          None => {}
        }
      } else if timer_message == TimerMessageType::writer_cache_cleaning {
        let found_writer_to_clean_some_cache = self
          .writers
          .iter_mut()
          .find(|p| p.1.get_timed_event_entity_token() == event.token());
        match found_writer_to_clean_some_cache {
          Some((_guid, w)) => {
            w.handle_cache_cleaning();
          }
          None => {}
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
        found_writer.handle_ack_nack(acknack_sender_prefix, acknack_message)
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
        for (_, writer) in self.writers.iter_mut() {
          if writer.get_entity_id() == EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER {
            DPEventWrapper::update_spdp_participant_readers(
              writer,
              &db,
              self.domain_info.domain_id,
            );

            if needs_new_cache_change {
              for proxy in writer.readers.iter_mut() {
                proxy.unsend_changes_set(writer.last_change_sequence_number);
              }
            }
          } else if writer.get_entity_id() == EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER {
            DPEventWrapper::update_pubsub_readers(
              writer,
              &db,
              EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER,
              BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR,
            );
            if needs_new_cache_change {
              for proxy in writer.readers.iter_mut() {
                proxy.unsend_changes_set(writer.last_change_sequence_number);
              }
            }
          } else if writer.get_entity_id() == EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER {
            DPEventWrapper::update_pubsub_readers(
              writer,
              &db,
              EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
              BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR,
            );
            if needs_new_cache_change {
              for proxy in writer.readers.iter_mut() {
                proxy.unsend_changes_set(writer.last_change_sequence_number);
              }
            }
          } else if writer.get_entity_id() == EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER {
            // TODO:
          } else {
            writer.readers = db
              .get_external_reader_proxies()
              .filter(|p| match p.subscription_topic_data.topic_name.as_ref() {
                Some(tn) => *writer.topic_name() == *tn,
                None => false,
              })
              .filter_map(|p| RtpsReaderProxy::from_discovered_reader_data(p))
              .collect();

            if let Some(Reliability::Reliable {
              max_blocking_time: _,
            }) = writer.get_qos().reliability
            {
              // reset data sending
              for reader in writer.readers.iter_mut() {
                reader.unsend_changes_set(writer.last_change_sequence_number);
              }
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
      DPEventWrapper::add_reader_to_writer(writer, reader);
    }

    // adding multicast reader
    let multicast_guid = GUID::new_with_prefix_and_id(
      GuidPrefix::GUIDPREFIX_UNKNOWN,
      EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
    );

    let mut multicast_reader = RtpsReaderProxy::new(multicast_guid);
    multicast_reader.multicast_locator_list =
      get_local_multicast_locators(get_spdp_well_known_multicast_port(domain_id));

    DPEventWrapper::add_reader_to_writer(writer, multicast_reader);
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
      DPEventWrapper::add_reader_to_writer(writer, reader);
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

  fn add_reader_to_writer(writer: &mut Writer, mut proxy: RtpsReaderProxy) {
    let reader = writer
      .readers
      .iter_mut()
      .find(|r| r.remote_reader_guid == proxy.remote_reader_guid);

    proxy.unsend_changes_set(writer.last_change_sequence_number);

    match reader {
      Some(r) => {
        if !r.content_is_equal(&proxy) {
          *r = proxy
        }
      }
      None => writer.readers.push(proxy),
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
          DPEventWrapper::update_pubsub_writers(
            reader,
            &db,
            EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER,
          );
        }
        EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER => {
          let proxies: Vec<RtpsWriterProxy> = db
            .get_participants()
            .filter(|sp| match sp.available_builtin_endpoints {
              Some(ep) => {
                ep.contains(BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER)
              }
              None => false,
            })
            .filter(|p| p.participant_guid.unwrap().guidPrefix != reader.get_guid_prefix())
            .map(|p| {
              p.as_writer_proxy(
                true,
                Some(EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER),
              )
            })
            .collect();
          reader.retain_matched_writers(proxies.iter());
          for proxy in proxies.into_iter() {
            reader.add_writer_proxy(proxy);
          }
        }
        _ => {
          let topic_name = reader.topic_name().clone();
          let proxies: Vec<RtpsWriterProxy> = db
            .get_external_writer_proxies()
            .filter(|p| match p.publication_topic_data.topic_name.as_ref() {
              Some(tn) => topic_name == *tn,
              None => false,
            })
            .filter_map(|p| RtpsWriterProxy::from_discovered_writer_data(p))
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
              Some(_) => TopicKind::WITH_KEY,
              None => TopicKind::NO_KEY,
            };
            let topic_data_type = match &topic.topic_data.type_name {
              Some(tn) => tn.clone(),
              None => continue,
            };
            ddsc.add_new_topic(topic_name, topic_kind, &TypeDesc::new(topic_data_type));
          }
        }
        _ => panic!("DDSCache is poisoned"),
      },
      _ => panic!("DiscoveryDB is poisoned"),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::thread;
  use std::time::Duration;
  use mio::{Ready, PollOpt};
  use crate::structure::entity::Entity;
  use crate::structure::dds_cache::DDSCache;

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

    let dp_event_wrapper = DPEventWrapper::new(
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
    dp_event_wrapper
      .poll
      .register(
        &receiver_stop,
        STOP_POLL_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register receivers.");

    let child = thread::spawn(move || DPEventWrapper::event_loop(dp_event_wrapper));

    let n = 3;

    let mut reader_guids = Vec::new();
    for i in 0..n {
      let new_guid = GUID::new();

      let (send, _rec) = mio_channel::sync_channel::<()>(100);
      let new_reader = Reader::new(
        new_guid,
        send,
        Arc::new(RwLock::new(DDSCache::new())),
        "test".to_string(),
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
}
