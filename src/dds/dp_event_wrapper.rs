use mio::{Poll, Event, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;
extern crate chrono;
//use chrono::Duration;
use std::collections::HashMap;
use std::{
  sync::{Arc, RwLock},
};

use crate::dds::message_receiver::MessageReceiver;
use crate::dds::reader::Reader;
use crate::dds::writer::Writer;
use crate::network::udp_listener::UDPListener;
use crate::network::constant::*;
use crate::structure::guid::{GuidPrefix, GUID, EntityId};
use crate::structure::entity::Entity;
use crate::{
  common::heartbeat_handler::HeartbeatHandler,
  discovery::discovery_db::DiscoveryDB,
  structure::{cache_change::ChangeKind, dds_cache::DDSCache},
  submessages::AckNack
};
//use crate::{common::heartbeat_handler::HeartbeatHandler, structure::{cache_change::ChangeKind, dds_cache::{DDSCache, DDSHistoryCache}}, submessages::AckNack};


pub struct DPEventWrapper {
  domain_participants_guid : GUID,
  poll: Poll,
  ddscache: Arc<RwLock<DDSCache>>,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  udp_listeners: HashMap<Token, UDPListener>,
  send_targets: HashMap<Token, mio_channel::Sender<Vec<u8>>>,
  message_receiver: MessageReceiver,

  // Adding readers
  add_reader_receiver: TokenReceiverPair<Reader>,
  remove_reader_receiver: TokenReceiverPair<GUID>,

  // Writers
  add_writer_receiver: TokenReceiverPair<Writer>,
  remove_writer_receiver: TokenReceiverPair<GUID>,
  writer_heartbeat_recievers: HashMap<Token, mio_channel::Receiver<Token>>,

  stop_poll_receiver: mio_channel::Receiver<()>,
  // GuidPrefix sent in this channel needs to be RTPSMessage source_guid_prefix. Writer needs this to locate RTPSReaderProxy if negative acknack.
  ack_nack_reciever: mio_channel::Receiver<(GuidPrefix,AckNack)>,

  writers: HashMap<GUID, Writer>,

  reader_update_notification_receiver: mio_channel::Receiver<()>,
  writer_update_notification_receiver: mio_channel::Receiver<()>,
}

impl DPEventWrapper {
  // This pub(crate) , because it should be constructed only by DomainParticipant.
  pub(crate) fn new(
    domain_participants_guid: GUID,
    udp_listeners: HashMap<Token, UDPListener>,
    ddscache: Arc<RwLock<DDSCache>>,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    send_targets: HashMap<Token, mio_channel::Sender<Vec<u8>>>,
    participant_guid_prefix: GuidPrefix,
    add_reader_receiver: TokenReceiverPair<Reader>,
    remove_reader_receiver: TokenReceiverPair<GUID>,
    add_writer_receiver: TokenReceiverPair<Writer>,
    remove_writer_receiver: TokenReceiverPair<GUID>,
    stop_poll_receiver: mio_channel::Receiver<()>,
    reader_update_notification_receiver: mio_channel::Receiver<()>,
    writer_update_notification_receiver: mio_channel::Receiver<()>,
  ) -> DPEventWrapper {
    let poll = Poll::new().expect("Unable to create new poll.");
    let (acknack_sender, acknack_reciever) = mio_channel::channel::<(GuidPrefix,AckNack)>();
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
      println!("registered listener with token:  {:?} on socket: {:?}", token, listener.mio_socket())
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

    poll.register(&acknack_reciever, ACKNACK_MESSGAGE_TO_LOCAL_WRITER_TOKEN, Ready::readable(), PollOpt::edge())
    .expect("Failed to register AckNack submessage sending from MessageReciever to DPEventLoop");

    poll
      .register(
        &reader_update_notification_receiver,
        READER_UPDATE_NOTIFICATION_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register reader update notification.");

    poll
      .register(
        &writer_update_notification_receiver,
        WRITER_UPDATE_NOTIFICATION_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register writer update notification token.");

    DPEventWrapper {
      domain_participants_guid,
      poll,
      ddscache,
      discovery_db,
      udp_listeners,
      send_targets,
      message_receiver: MessageReceiver::new(participant_guid_prefix,acknack_sender),
      add_reader_receiver,
      remove_reader_receiver,
      add_writer_receiver,
      remove_writer_receiver,
      writer_heartbeat_recievers: HashMap::new(),
      stop_poll_receiver,
      writers: HashMap::new(),
      ack_nack_reciever : acknack_reciever,
      reader_update_notification_receiver,
      writer_update_notification_receiver,
    }
  }

  pub fn event_loop(self) {
    // TODO: Use the dp to access stuff we need, e.g. historycache
    let mut ev_wrapper = self;
    loop {
      let mut events = Events::with_capacity(1024);
      ev_wrapper
        .poll
        .poll(&mut events, None)
        .expect("Failed in waiting of poll.");
      for event in events.into_iter() {
        println!("Dp_eventwrapper poll received: {:?}", event); // for debugging!!!!!!

        if event.token() == STOP_POLL_TOKEN {
          return;
        } else if DPEventWrapper::is_udp_traffic(&event) {
          ev_wrapper.handle_udp_traffic(&event);
        } else if DPEventWrapper::is_reader_action(&event) {
          ev_wrapper.handle_reader_action(&event);
        } else if DPEventWrapper::is_writer_action(&event) {
          ev_wrapper.handle_writer_action(&event);
        } else if ev_wrapper.is_writer_heartbeat_action(&event){
          ev_wrapper.handle_writer_heartbeat(&event);
        } else if DPEventWrapper::is_writer_acknack_action(&event){
          ev_wrapper.handle_writer_acknack_action(&event);
        } else if DPEventWrapper::is_reader_update_notification(&event) {
          // TODO:
        } else if DPEventWrapper::is_writer_update_notification(&event) {
          // TODO:
        }
      }
    }
  }

  pub fn is_udp_traffic(event: &Event) -> bool {
    event.token() == DISCOVERY_SENDER_TOKEN || event.token() == USER_TRAFFIC_SENDER_TOKEN || event.token() == USER_TRAFFIC_LISTENER_TOKEN ||  event.token() == USER_TRAFFIC_MUL_LISTENER_TOKEN

  }

  pub fn is_reader_action(event: &Event) -> bool {
    event.token() == ADD_READER_TOKEN || event.token() == REMOVE_READER_TOKEN
  }

  /// Writer action can be add writer remove writer or some not predefined token.
  /// if not predefined token -> EntityIdToken can be calculated and if entityKind is 0xC2 then it is writer action.
  pub fn is_writer_action(event: &Event) -> bool {
    if EntityId::from_usize(event.token().0).is_some() {
      let maybeWriterKind: EntityId = EntityId::from_usize(event.token().0).unwrap();
      if maybeWriterKind.get_kind() == 0xC2 {
        return true;
      }
    }
    event.token() == ADD_WRITER_TOKEN || event.token() == REMOVE_WRITER_TOKEN
  }

  pub fn is_writer_heartbeat_action(&self, event: &Event) -> bool {
    if self.writer_heartbeat_recievers.contains_key(&event.token()) {
      return true;
    }
    return false;
  }

  pub fn is_writer_acknack_action(event: &Event) -> bool{
    event.token() == ACKNACK_MESSGAGE_TO_LOCAL_WRITER_TOKEN 
  }

  pub fn is_reader_update_notification(event: &Event) -> bool {
    event.token() == READER_UPDATE_NOTIFICATION_TOKEN
  }

  pub fn is_writer_update_notification(event: &Event) -> bool {
    event.token() == WRITER_UPDATE_NOTIFICATION_TOKEN
  }

  pub fn handle_udp_traffic(&mut self, event: &Event) {
    println!("handle udp traffic");
    let listener = self.udp_listeners.get(&event.token());
    match listener {
      Some(l) => loop {
        let data = l.get_message();
        if data.is_empty() {
          //println!("UDP data is empty!");
          return;
        }

        if event.token() == DISCOVERY_LISTENER_TOKEN {
          self.message_receiver.handle_discovery_msg(data);
        } else if event.token() == USER_TRAFFIC_LISTENER_TOKEN {
          self.message_receiver.handle_user_msg(data);
        }
      },
      None => {print!("Cannot handle upd traffic! No listener with token {:?}", &event.token());
      return;
      }
    };
  }

  pub fn handle_reader_action(&mut self, event: &Event) {
    match event.token() {
      ADD_READER_TOKEN => {
        let new_reader = self
          .add_reader_receiver
          .receiver
          .try_recv()
          .expect("Can't receive new reader");
        self.message_receiver.add_reader(new_reader);
      }
      REMOVE_READER_TOKEN => {
        let old_reader_guid = self.remove_reader_receiver.receiver.try_recv().unwrap();
        self.message_receiver.remove_reader(old_reader_guid);
      }
      _ => {}
    }
  }

  pub fn handle_writer_action(&mut self, event: &Event) {
    println!("dp_ew handle writer action with token {:?}", event.token());
    match event.token() {
      ADD_WRITER_TOKEN => {
        let mut new_writer = self
          .add_writer_receiver
          .receiver
          .try_recv()
          .expect("Failed to receive new add writer message.");
        &self.poll.register(
          new_writer.cache_change_receiver(),
          new_writer.get_entity_token(),
          Ready::readable(),
          PollOpt::edge(),
        );

        let (hearbeatSender, hearbeatReciever) = mio_channel::channel::<Token>();
        let heartBeatHandler: HeartbeatHandler = HeartbeatHandler::new(
          hearbeatSender.clone(),
          new_writer.get_heartbeat_entity_token(),
        );
        new_writer.add_heartbeat_handler(heartBeatHandler);
        self
          .poll
          .register(
            &hearbeatReciever,
            new_writer.get_heartbeat_entity_token(),
            Ready::readable(),
            PollOpt::edge(),
          )
          .expect("Writer heartbeat timer channel registeration failed!!");
        self
          .writer_heartbeat_recievers
          .insert(new_writer.get_heartbeat_entity_token(), hearbeatReciever);
        self.writers.insert(new_writer.as_entity().guid, new_writer);
      }
      REMOVE_WRITER_TOKEN => {
        let writer = self.writers.remove(
          &self
            .remove_writer_receiver
            .receiver
            .try_recv()
            .expect("Failed to get writer reciver guid"),
        );
        if let Some(w) = writer {
          &self.poll.deregister(w.cache_change_receiver());
        };
      }
      t => {
        let found_writer = self.writers.iter_mut().find(|p| p.1.get_entity_token() == t);

        match found_writer {
          Some((_guid, w)) => {
            let cache_change = w.cache_change_receiver().try_recv();
            println!("found RTPS writer with entity token {:?}", t);

            match cache_change {
              Ok(cc) => {
                println!("Change Kind: {:?}", cc.change_kind);
                if cc.change_kind == ChangeKind::NOT_ALIVE_DISPOSED {
                  w.handle_not_alive_disposed_cache_change(cc);
                } else if cc.change_kind == ChangeKind::ALIVE {
                  w.insert_to_history_cache(cc);
                  w.send_all_unsend_messages();
                }
              }
              _ => (),
            }
          }
          None => {}
        }
      }
    }
  }

  pub fn handle_writer_heartbeat(&mut self, event: &Event/*, writers: &mut HashMap<GUID, Writer>*/){
    println!("is writer heartbeat action!");
    let found_writer_with_heartbeat = self.writers.iter_mut().find(|p| p.1.get_heartbeat_entity_token() == event.token());
    match found_writer_with_heartbeat {
      Some((_guid, w)) => {
        w.handle_heartbeat_tick();
        }
      None => {}
    }
    let found_writer_with_heartbeat = self.writers.iter_mut().find(|p| p.1.get_heartbeat_entity_token() == event.token());
    match found_writer_with_heartbeat {
      Some((_guid, w)) => {
        w.handle_heartbeat_tick();
      }
      None => {}
    }
  }


  pub fn handle_writer_acknack_action(&mut self, _event: &Event){
    println!("is writer acknack action!");
    let recieved = self.ack_nack_reciever.try_recv();
   
    if recieved.is_ok(){
      let (acknack_sender_prefix, acknack_message) = recieved.unwrap();
      let target_writer_entity_id = {
        acknack_message.writer_id
      };
      let writer_guid = GUID::new_with_prefix_and_id(self.domain_participants_guid.guidPrefix, target_writer_entity_id);
      let found_writer = self.writers.get_mut(&writer_guid);
      if found_writer.is_some(){
        found_writer.unwrap().handle_ack_nack(&acknack_sender_prefix, acknack_message)
      }
      else{
        panic!("Couldn't handle acknack! did not find local rtps writer with GUID: {:?}", writer_guid);
      }
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
  use std::time::Instant;
  //use std::sync::mpsc;

  #[test]
  fn dpew_add_and_remove_readers() {
    // Adding readers
    let (sender_add_reader, receiver_add) = mio_channel::channel::<Reader>();
    let (sender_remove_reader, receiver_remove) = mio_channel::channel::<GUID>();

    let (_add_writer_sender, add_writer_receiver) = mio_channel::channel();
    let (_remove_writer_sender, remove_writer_receiver) = mio_channel::channel();

    let (_stop_poll_sender, stop_poll_receiver) = mio_channel::channel();

    let (_reader_update_notification_sender, reader_update_notification_receiver) =
      mio_channel::channel();
    let (_writer_update_notification_sender, writer_update_notification_receiver) =
      mio_channel::channel();

    let ddshc = Arc::new(RwLock::new(DDSCache::new()));
    let discovery_db = Arc::new(RwLock::new(DiscoveryDB::new()));

    let domain_participant_guid = GUID::new();
    let dp_event_wrapper = DPEventWrapper::new(
      domain_participant_guid,
      HashMap::new(),
      ddshc,
      discovery_db,
      HashMap::new(),
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
      reader_update_notification_receiver,
      writer_update_notification_receiver,
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

      let (send, _rec) = mio_channel::sync_channel::<Instant>(100);
      let new_reader = Reader::new(
        new_guid,
        send,
        Arc::new(RwLock::new(DDSCache::new())),
        "test".to_string(),
      );

      reader_guids.push(new_reader.get_guid().clone());
      println!("\nSent reader number {}: {:?}\n", i, &new_reader);
      sender_add_reader.send(new_reader).unwrap();
      std::thread::sleep(Duration::new(0, 100));
    }

    println!("\npoistetaan toka\n");
    let some_guid = reader_guids[1].clone();
    sender_remove_reader.send(some_guid).unwrap();
    std::thread::sleep(Duration::new(0, 100));

    println!("\nLopetustoken lähtee\n");
    sender_stop.send(0).unwrap();
    child.join().unwrap();
  }
}
