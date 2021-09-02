//use std::cmp::max;

use crate::discovery::data_types::topic_data::DiscoveredWriterData;
use crate::discovery::data_types::topic_data::DiscoveredReaderData;
use crate::discovery::data_types::spdp_participant_data::SPDPDiscoveredParticipantData;
use crate::discovery::discovery::Discovery;

use log::{debug, error, info, warn, trace};
use mio::{Poll, Event, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;
use std::{collections::HashMap, time::Duration};
use std::{
  sync::{Arc, RwLock},
};

use crate::{
  dds::{message_receiver::MessageReceiver, reader::Reader, writer::Writer},
  network::util::get_local_multicast_locators,
  structure::builtin_endpoint::{BuiltinEndpointSet, },
  dds::qos::policy,
};
use crate::network::udp_listener::UDPListener;
use crate::network::constant::*;
use crate::structure::guid::{GuidPrefix, GUID, EntityId, TokenDecode};
use crate::structure::entity::RTPSEntity;

use crate::{
  common::timed_event_handler::{TimedEventHandler},
  discovery::discovery_db::DiscoveryDB,
  structure::{dds_cache::DDSCache, topic_kind::TopicKind},
  messages::submessages::submessages::AckNack,
};

use super::{
  rtps_reader_proxy::RtpsReaderProxy, rtps_writer_proxy::RtpsWriterProxy,
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

  // Writers
  add_writer_receiver: TokenReceiverPair<Writer>,
  remove_writer_receiver: TokenReceiverPair<GUID>,
  writer_timed_event_reciever: HashMap<Token, mio_channel::Receiver<TimerMessageType>>,

  stop_poll_receiver: mio_channel::Receiver<()>,
  // GuidPrefix sent in this channel needs to be RTPSMessage source_guid_prefix. Writer needs this to locate RTPSReaderProxy if negative acknack.
  ack_nack_reciever: mio_channel::Receiver<(GuidPrefix, AckNack)>,

  writers: HashMap<EntityId, Writer>,

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

    let mut ev_wrapper = self;
    loop {
      ev_wrapper.poll.poll(&mut events, None)
        .expect("Failed in waiting of poll.");

      for event in events.iter() {
        match EntityId::from_token( event.token() ) {
          TokenDecode::FixedToken(fixed_token) =>
            match fixed_token {
              STOP_POLL_TOKEN => {
                info!("Stopping dp_event_loop");
                return
              }
              DISCOVERY_LISTENER_TOKEN |
              DISCOVERY_MUL_LISTENER_TOKEN |
              USER_TRAFFIC_LISTENER_TOKEN |
              USER_TRAFFIC_MUL_LISTENER_TOKEN => {
                let udp_messages = ev_wrapper.udp_listeners.get_mut(&event.token())
                  .map_or_else(
                      | | { error!("No listener with token {:?}", &event.token() ); vec![] }, 
                      |l| l.get_messages()
                    );
                for packet in udp_messages.into_iter() {
                  ev_wrapper.message_receiver.handle_received_packet(packet)
                }
              }
              ADD_READER_TOKEN | REMOVE_READER_TOKEN => {
                ev_wrapper.handle_reader_action(&event);
              }
              ADD_WRITER_TOKEN | REMOVE_WRITER_TOKEN => {
                ev_wrapper.handle_writer_action(&event); 
              }
              ACKNACK_MESSGAGE_TO_LOCAL_WRITER_TOKEN => {
                ev_wrapper.handle_writer_acknack_action(&event);
              }
              DISCOVERY_UPDATE_NOTIFICATION_TOKEN => {
                while let Ok(dnt) = ev_wrapper.discovery_update_notification_receiver.try_recv() {
                  use DiscoveryNotificationType::*;
                  match dnt {
                    WriterUpdated{ discovered_writer_data } => 
                      ev_wrapper.remote_writer_discovered(discovered_writer_data),

                    WriterLost{writer_guid} => ev_wrapper.remote_writer_lost(writer_guid) ,

                    ReaderUpdated{ discovered_reader_data, rtps_reader_proxy, _needs_new_cache_change } => 
                      ev_wrapper.remote_reader_discovered(discovered_reader_data, rtps_reader_proxy, _needs_new_cache_change),

                    ReaderLost{ reader_guid } => ev_wrapper.remote_reader_lost(reader_guid) ,

                    ParticipantUpdated{ guid_prefix } => ev_wrapper.update_participant(guid_prefix),
                      
                    ParticipantLost{ guid_prefix } => ev_wrapper.remote_participant_lost(guid_prefix),

                    TopicsInfoUpdated => ev_wrapper.update_topics(),
                    AssertTopicLiveliness{ writer_guid , manual_assertion } => {
                      ev_wrapper.writers.get_mut(&writer_guid.entityId)
                        .map( |w| w.handle_heartbeat_tick(manual_assertion) ); 
                    }
                  }
                }              
              }
              DPEV_ACKNACK_TIMER_TOKEN => {
                ev_wrapper.message_receiver.send_preemptive_acknacks();
                acknack_timer.set_timeout(PREEMPTIVE_ACKNACK_PERIOD, ());
              }

              fixed_unknown => {
                error!("Unknown event.token {:?} = 0x{:x?} , decoded as {:?}", 
                  event.token(), event.token().0, fixed_unknown );
              }
            },

          // Commands/actions
          TokenDecode::Entity( eid ) => 
            if eid.kind().is_reader() {
              ev_wrapper.message_receiver.get_reader_mut( eid )
                .map( |reader| reader.process_command() )
                .unwrap_or_else(|| error!("Event for unknown reader {:?}",eid));
            } else if eid.kind().is_writer() {
              ev_wrapper.writers.get_mut( &eid )
                .map( |writer| writer.process_writer_command() )
                .unwrap_or_else(|| error!("Event for unknown writer {:?}",eid));
            } else { 
              error!("Entity Event for unknown EntityKind {:?}",eid); 
            },
            
          // Timed Actions
          TokenDecode::AltEntity( eid ) =>
            if eid.kind().is_reader() { 
              ev_wrapper.handle_reader_timed_event(&event); 
            } else if eid.kind().is_writer() {
              ev_wrapper.handle_writer_timed_event(&event);
            }
            else { error!("AltEntity Event for unknown EntityKind {:?}",eid); },
        }
      } // for    

    } // loop
  } // fn

  fn handle_reader_action(&mut self, event: &Event) {
    match event.token() {
      ADD_READER_TOKEN => {
        trace!("add reader(s)");
        while let Ok(mut new_reader) = self.add_reader_receiver.receiver.try_recv() {
          let (timed_action_sender, timed_action_receiver) =
            mio_channel::sync_channel::<TimerMessageType>(10);
          let time_handler: TimedEventHandler = TimedEventHandler::new(timed_action_sender.clone());
          new_reader.add_timed_event_handler(time_handler);
          // Timed action polling
          self
            .poll
            .register(
              &timed_action_receiver,
              new_reader.get_reader_alt_entity_token(),
              Ready::readable(),
              PollOpt::edge(),
            )
            .expect("Reader timer channel registeration failed!");
          self
            .reader_timed_event_receiver
            .insert(new_reader.get_reader_alt_entity_token(), timed_action_receiver);
          // Non-timed action polling
          self
            .poll
            .register(
              &new_reader.data_reader_command_receiver,
              new_reader.get_entity_token(),
              Ready::readable(),
              PollOpt::edge(),
            )
            .expect("Reader command channel registration failed!!!");

          new_reader.set_requested_deadline_check_timer();
          trace!("Add reader: {:?}", new_reader);
          self.message_receiver.add_reader(new_reader);
        }
      }
      REMOVE_READER_TOKEN => {
        while let Ok(old_reader_guid) = self.remove_reader_receiver.receiver.try_recv() {
          if let Some(old_reader) = self.message_receiver.remove_reader(old_reader_guid) {
            if let Some(receiver) = 
              self.reader_timed_event_receiver.remove(&old_reader.get_reader_alt_entity_token()) {
                self.poll.deregister(&receiver)
                  .unwrap_or_else(|e| error!("reader_timed_event_receiver deregister: {:?}",e));
            } else {
              warn!("Reader had no reader_timed_event_receiver? {:?}", old_reader_guid);
            }
            self.poll.deregister( &old_reader.data_reader_command_receiver )
              .unwrap_or_else(|e| error!("Cannot deregister data_reader_command_receiver: {:?}",e));
          } else {
            warn!("Tried to remove nonexistent Reader {:?}",old_reader_guid);
          }
        }
      }
      _ => {}
    }
  }

  fn handle_writer_action(&mut self, event: &Event) {
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
          self.writers.insert(new_writer.get_guid().entityId, new_writer);
        }
      }
      REMOVE_WRITER_TOKEN => {
        while let Ok(writer_guid) = &self.remove_writer_receiver.receiver.try_recv() {
          let writer = self.writers.remove(&writer_guid.entityId);
          
          if let Some(w) = writer {
            self.poll.deregister(&w.writer_command_receiver)
              .unwrap_or_else( |e| error!("Deregister fail {:?}",e));

            if let Some(rec) = 
                self.writer_timed_event_reciever.remove( &w.get_timed_event_entity_token() ) {
              self.poll.deregister(&rec)
                .unwrap_or_else( |e| error!("Deregister fail {:?}",e));
            }
          }
        }
      }
      other => error!("Expected writer action token, got {:?}",other),
    }
  }

  /// Writer timed events can be heatrbeats or cache cleaning events.
  /// events are distinguished by TimerMessageType which is send via mio channel. Channel token in
  fn handle_writer_timed_event(&mut self, event: &Event) {
    let reciever = self.writer_timed_event_reciever.get(&event.token())
      .expect("Did not find a heartbeat receiver");
    while let Ok(timer_message) = reciever.try_recv() {
      self.writers.iter_mut()
          .find(|(_guid,writer)| writer.get_timed_event_entity_token() == event.token())
          .map( |(_guid, writer)| writer.handle_timed_event(timer_message));
    }
  }

  fn handle_reader_timed_event(&mut self, event: &Event) {
    let reciever = self.reader_timed_event_receiver.get(&event.token())
      .expect("Did not found reader timed event reciever!");
    // TODO: Why do we have separate message channels for reader and writer, if they have contain the same data type?

    while let Ok(timer_message) = reciever.try_recv() {
      match self.message_receiver.available_readers.values_mut()
          .find(|reader| reader.get_entity_token() == event.token()) {
        Some(reader) => {
          reader.handle_timed_event(timer_message);
        }
        None => error!("Reader was not found with entity token {:?}",  event.token()),
      }
    }
  }

  fn handle_writer_acknack_action(&mut self, _event: &Event) {
    while let Ok((acknack_sender_prefix, acknack_message)) = self.ack_nack_reciever.try_recv() {
      let writer_guid = GUID::new_with_prefix_and_id(
        self.domain_info.domain_participant_guid.guidPrefix,
        acknack_message.writer_id,
      );
      if let Some(found_writer) = self.writers.get_mut(&writer_guid.entityId) {
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

  fn update_participant(&mut self, participant_guid_prefix: GuidPrefix ) {
    info!("update_participant - begin for {:?}", participant_guid_prefix);
    if participant_guid_prefix == self.domain_info.domain_participant_guid.guidPrefix {
      // Our own participant was updated (initialized.)
      // What should we do now?
      debug!("Own participant initialized");
    } else {
      let db = self.discovery_db.read().unwrap();
      // new Remote Participant discovered
      let discovered_participant = 
        match db.find_participant_proxy(participant_guid_prefix) {
          Some(dpd) => dpd,
          None => {
            error!("Participant was updated, but DB does not have it. Strange."); 
            return
          }
        };

      for (writer_eid, reader_eid, endpoint) in 
      & [ ( EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER, // SPDP
            EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR )

        , ( EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER, // SEDP ...
            EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR )

        , ( EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
            EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR ) 

        , ( EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER,
            EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_TOPICS_DETECTOR )

        , ( EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
            EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER,
            BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_MESSAGE_DATA_READER)
        ] {
        if let Some(writer) = self.writers.get_mut( writer_eid ) {
          debug!("update_discovery_writer - {:?}", writer.topic_name() );
          let mut qos = Discovery::subscriber_qos();
          // special case by RTPS 2.3 spec Section 
          // "8.4.13.3 BuiltinParticipantMessageWriter and 
          // BuiltinParticipantMessageReader QoS"
          if *reader_eid == EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER 
              && discovered_participant.builtin_endpoint_qos
                  .map( |beq| beq.is_best_effort() )
                  .unwrap_or(false) {                
              qos.reliability = Some(policy::Reliability::BestEffort);
          };

          if discovered_participant.available_builtin_endpoints.contains(*endpoint) {
            let mut reader_proxy = discovered_participant.as_reader_proxy(true, Some(*reader_eid));

            if *writer_eid == EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER {
              // Simple Particiapnt Discovery Protocol (SPDP) writer is special,
              // different from SEDP writers
              qos = Discovery::create_spdp_patricipant_qos(); // different QoS
              // adding a multicast reader
              reader_proxy.remote_reader_guid = GUID::new_with_prefix_and_id(
                GuidPrefix::GUIDPREFIX_UNKNOWN,
                EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER);

              reader_proxy.multicast_locator_list =
                get_local_multicast_locators(get_spdp_well_known_multicast_port(self.domain_info.domain_id));
            }
            // common processing for SPDP and SEDP
            writer.update_reader_proxy( reader_proxy , qos );
            debug!("update_discovery writer - endpoint {:?} - {:?}", 
              endpoint, discovered_participant.participant_guid);
          }

          writer.notify_new_data_to_all_readers() 
        }
      }
      // update local readers.
      // list to be looped over is the same as above, but now
      // EntityIds are for announcers
      for (writer_eid, reader_eid, endpoint) in 
      & [ ( EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER, // SPDP
            EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PARTICIPANT_ANNOUNCER )

        , ( EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER, // SEDP ...
            EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER )

        , ( EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
            EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER ) 

        , ( EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER,
            EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER,
            BuiltinEndpointSet::DISC_BUILTIN_ENDPOINT_TOPICS_ANNOUNCER )

        , ( EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
            EntityId::ENTITYID_P2P_BUILTIN_PARTICIPANT_MESSAGE_READER,
            BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_MESSAGE_DATA_WRITER)
        ] 
      {
        if let Some(reader) = self.message_receiver.available_readers
            .get_mut( reader_eid ) {
          debug!("try update_discovery_reader - {:?}", reader.topic_name() );
          let qos = 
            if *reader_eid == EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER 
              { Discovery::create_spdp_patricipant_qos() } 
            else { Discovery::publisher_qos() };
          let wp = discovered_participant.as_writer_proxy(true, Some(*writer_eid));

          if discovered_participant.available_builtin_endpoints.contains(*endpoint) {
            reader.update_writer_proxy( wp , qos );
            debug!("update_discovery_reader - endpoint {:?} - {:?}", 
              *endpoint, discovered_participant.participant_guid);
          }
        }
      } // for
    } // if
    info!("update_participant - finished for {:?}", participant_guid_prefix);
  } // fn 

  fn remote_participant_lost(&mut self, participant_guid_prefix: GuidPrefix ) {
    info!("remote_participant_lost guidPrefix={:?}", &participant_guid_prefix );
    // Discovery has already removed Particiapnt from Discovery DB
    // Now we have to remove any ReaderProxies and WriterProxies belonging
    // to that particiapnt, so that we do not send messages to them anymore.

    for writer in self.writers.values_mut() {
      writer.participant_lost(participant_guid_prefix)
    }

    for reader in self.message_receiver.available_readers.values_mut() {
      reader.participant_lost(participant_guid_prefix)
    }
  }

  fn remote_reader_discovered(&mut self, drd: DiscoveredReaderData, 
      rtps_reader_proxy: RtpsReaderProxy , _needs_new_cache_change: bool) {
    for (_writer_guid, writer) in self.writers.iter_mut() {
      if drd.subscription_topic_data.topic_name() == writer.topic_name() {
        writer.update_reader_proxy(rtps_reader_proxy.clone(), 
          drd.subscription_topic_data.generate_qos());
      }
    }
  }

  fn remote_reader_lost(&mut self, reader_guid:GUID) {
    for (_writer_guid, writer) in self.writers.iter_mut() {
      writer.reader_lost(reader_guid);
    }
  }

  fn update_discovery_reader(reader: &mut Reader, dpd: &SPDPDiscoveredParticipantData,
    entity_id: EntityId, expected_endpoint: u32 ) 
  {
    debug!("update_discovery_reader - {:?}", reader.topic_name() );
    if dpd.available_builtin_endpoints.contains(expected_endpoint) {
      reader.update_writer_proxy( 
        dpd.as_writer_proxy(true, Some(entity_id)),
        Discovery::publisher_qos()
       );
      debug!("update_discovery reader - endpoint {:?} - {:?}", expected_endpoint, dpd.participant_guid);
    }
  }

  fn remote_writer_discovered(&mut self, dwd: DiscoveredWriterData) {
    for reader in self.message_receiver.available_readers.values_mut() {
      if &dwd.publication_topic_data.topic_name == reader.topic_name() {
        reader.update_writer_proxy( 
          RtpsWriterProxy::from_discovered_writer_data(&dwd),
          dwd.publication_topic_data.qos(), 
        );
      }
    } 
  }  

  fn remote_writer_lost(&mut self, writer_guid: GUID) {
    for reader in self.message_receiver.available_readers.values_mut() {
      reader.remove_writer_proxy( writer_guid );
    }
  }

  fn update_topics(&mut self) {
    match self.discovery_db.read() {
      Ok(db) => match self.ddscache.write() {
        Ok(mut ddsc) => {
          for topic in db.get_all_topics() {
            // TODO: how do you know when topic is keyed and is not
            let topic_kind = match &topic.topic_data.key {
              Some(_) => TopicKind::WithKey,
              None => TopicKind::NoKey,
            };
            ddsc.add_new_topic(&topic.topic_data.name, topic_kind,
               TypeDesc::new(&topic.topic_data.type_name));
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
    dds::statusevents::DataReaderStatus,
  };
  use mio::{Ready, PollOpt};
  use crate::{
    dds::with_key::datareader::ReaderCommand,
    structure::entity::RTPSEntity,
    dds::qos::QosPolicies,
  };
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
    let discovery_db = Arc::new(RwLock::new(DiscoveryDB::new(GUID::new_particiapnt_guid())));

    let domain_info = DomainInfo {
      domain_participant_guid: GUID::default(),
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
      let new_guid = GUID::default();

      let (send, _rec) = mio_channel::sync_channel::<()>(100);
      let (status_sender, _status_reciever) =
        mio_extras::channel::sync_channel::<DataReaderStatus>(100);
      let (_reader_commander, reader_command_receiver) =
        mio_extras::channel::sync_channel::<ReaderCommand>(100);

      let new_reader = Reader::new(
        new_guid,
        send,
        status_sender,
        Arc::new(RwLock::new(DDSCache::new())),
        "test".to_string(),
        QosPolicies::qos_none(),
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
  

  // TODO: Rewrite / remove this test - all asserts in it use
  // DataReader::get_requested_deadline_missed_status which is
  // currently commented out

  // #[test]
  // fn dpew_test_reader_commands() {
  //   let somePolicies = QosPolicies {
  //     durability: None,
  //     presentation: None,
  //     deadline: Some(Deadline(DurationDDS::from_millis(500))),
  //     latency_budget: None,
  //     ownership: None,
  //     liveliness: None,
  //     time_based_filter: None,
  //     reliability: None,
  //     destination_order: None,
  //     history: None,
  //     resource_limits: None,
  //     lifespan: None,
  //   };
  //   let dp = DomainParticipant::new(0).expect("Failed to create participant");
  //   let sub = dp.create_subscriber(&somePolicies).unwrap();

  //   let topic_1 = dp
  //     .create_topic("TOPIC_1", "jotain", &somePolicies, TopicKind::WithKey)
  //     .unwrap();
  //   let _topic_2 = dp
  //     .create_topic("TOPIC_2", "jotain", &somePolicies, TopicKind::WithKey)
  //     .unwrap();
  //   let _topic_3 = dp
  //     .create_topic("TOPIC_3", "jotain", &somePolicies, TopicKind::WithKey)
  //     .unwrap();

  //   // Adding readers
  //   let (sender_add_reader, receiver_add) = mio_channel::channel::<Reader>();
  //   let (_sender_remove_reader, receiver_remove) = mio_channel::channel::<GUID>();

  //   let (_add_writer_sender, add_writer_receiver) = mio_channel::channel();
  //   let (_remove_writer_sender, remove_writer_receiver) = mio_channel::channel();

  //   let (_stop_poll_sender, stop_poll_receiver) = mio_channel::channel();

  //   let (_discovery_update_notification_sender, discovery_update_notification_receiver) =
  //     mio_channel::channel();

  //   let ddshc = Arc::new(RwLock::new(DDSCache::new()));
  //   let discovery_db = Arc::new(RwLock::new(DiscoveryDB::new()));

  //   let domain_info = DomainInfo {
  //     domain_participant_guid: GUID::default(),
  //     domain_id: 0,
  //     participant_id: 0,
  //   };

  //   let dp_event_loop = DPEventLoop::new(
  //     domain_info,
  //     HashMap::new(),
  //     ddshc,
  //     discovery_db,
  //     GuidPrefix::default(),
  //     TokenReceiverPair {
  //       token: ADD_READER_TOKEN,
  //       receiver: receiver_add,
  //     },
  //     TokenReceiverPair {
  //       token: REMOVE_READER_TOKEN,
  //       receiver: receiver_remove,
  //     },
  //     TokenReceiverPair {
  //       token: ADD_WRITER_TOKEN,
  //       receiver: add_writer_receiver,
  //     },
  //     TokenReceiverPair {
  //       token: REMOVE_WRITER_TOKEN,
  //       receiver: remove_writer_receiver,
  //     },
  //     stop_poll_receiver,
  //     discovery_update_notification_receiver,
  //   );

  //   let (sender_stop, receiver_stop) = mio_channel::channel::<i32>();
  //   dp_event_loop
  //     .poll
  //     .register(
  //       &receiver_stop,
  //       STOP_POLL_TOKEN,
  //       Ready::readable(),
  //       PollOpt::edge(),
  //     )
  //     .expect("Failed to register receivers.");

  //   let child = thread::spawn(move || DPEventLoop::event_loop(dp_event_loop));

  //   //TODO IF THIS IS SET TO 1 TEST SUCCEEDS
  //   let n = 1;

  //   let mut reader_guids = Vec::new();
  //   let mut data_readers: Vec<DataReader<RandomData, CDRDeserializerAdapter<RandomData>>> = vec![];
  //   let _topics: Vec<Topic> = vec![];
  //   for i in 0..n {
  //     //topics.push(topic);
  //     let new_guid = GUID::default();

  //     let (send, _rec) = mio_channel::sync_channel::<()>(100);
  //     let (status_sender, status_reciever_DataReader) =
  //       mio_extras::channel::sync_channel::<DataReaderStatus>(1000);
  //     let (reader_commander, reader_command_receiver) =
  //       mio_extras::channel::sync_channel::<ReaderCommand>(1000);

  //     let mut new_reader = Reader::new(
  //       new_guid,
  //       send,
  //       status_sender,
  //       Arc::new(RwLock::new(DDSCache::new())),
  //       "test".to_string(),
  //       QosPolicies::qos_none(),
  //       reader_command_receiver,
  //     );

  //     let somePolicies = QosPolicies {
  //       durability: None,
  //       presentation: None,
  //       deadline: Some(Deadline(DurationDDS::from_millis(50))),
  //       latency_budget: None,
  //       ownership: None,
  //       liveliness: None,
  //       time_based_filter: None,
  //       reliability: None,
  //       destination_order: None,
  //       history: None,
  //       resource_limits: None,
  //       lifespan: None,
  //     };

  //     let mut datareader = sub
  //       .create_datareader::<RandomData, CDRDeserializerAdapter<RandomData>>(
  //         topic_1.clone(),
  //         Some(somePolicies.clone()),
  //       )
  //       .unwrap();

  //     datareader.set_status_change_receiver(status_reciever_DataReader);
  //     datareader.set_reader_commander(reader_commander);
  //     data_readers.push(datareader);

  //     //new_reader.set_qos(&somePolicies).unwrap();
  //     new_reader.matched_writer_add(GUID::default(), EntityId::ENTITYID_UNKNOWN, vec![], vec![]);
  //     reader_guids.push(new_reader.get_guid().clone());
  //     info!("\nSent reader number {}: {:?}\n", i, &new_reader);
  //     sender_add_reader.send(new_reader).unwrap();
  //     std::thread::sleep(Duration::from_millis(100));
  //   }
  //   thread::sleep(Duration::from_millis(100));

  //   let status = data_readers
  //     .get_mut(0)
  //     .unwrap()
  //     .get_requested_deadline_missed_status();
  //   info!("Received status change: {:?}", status);
  //   assert_eq!(
  //     status.unwrap(),
  //     Some(RequestedDeadlineMissedStatus::from_count(
  //       CountWithChange::start_from(3, 3)
  //     )),
  //   );
  //   thread::sleep(Duration::from_millis(150));

  //   let status2 = data_readers
  //     .get_mut(0)
  //     .unwrap()
  //     .get_requested_deadline_missed_status();
  //   info!("Received status change: {:?}", status2);
  //   assert_eq!(
  //     status2.unwrap(),
  //     Some(RequestedDeadlineMissedStatus::from_count(
  //       CountWithChange::start_from(6, 3)
  //     ))
  //   );

  //   let status3 = data_readers
  //     .get_mut(0)
  //     .unwrap()
  //     .get_requested_deadline_missed_status();
  //   info!("Received status change: {:?}", status3);
  //   assert_eq!(
  //     status3.unwrap(),
  //     Some(RequestedDeadlineMissedStatus::from_count(
  //       CountWithChange::start_from(6, 0)
  //     ))
  //   );

  //   thread::sleep(Duration::from_millis(50));

  //   let status4 = data_readers
  //     .get_mut(0)
  //     .unwrap()
  //     .get_requested_deadline_missed_status();
  //   info!("Received status change: {:?}", status4);
  //   assert_eq!(
  //     status4.unwrap(),
  //     Some(RequestedDeadlineMissedStatus::from_count(
  //       CountWithChange::start_from(7, 1)
  //     ))
  //   );

  //   info!("\nLopetustoken lähtee\n");
  //   sender_stop.send(0).unwrap();
  //   child.join().unwrap();
  // }
}
