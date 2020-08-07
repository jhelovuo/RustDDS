use mio::{Poll, Event, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;

use std::collections::HashMap;

use crate::dds::message_receiver::MessageReceiver;
use crate::dds::reader::Reader;
use crate::dds::writer::Writer;
use crate::network::udp_listener::UDPListener;
use crate::network::constant::*;
use crate::structure::guid::{GuidPrefix, GUID, EntityId};
use crate::structure::entity::Entity;

pub struct DPEventWrapper {
  poll: Poll,
  udp_listeners: HashMap<Token, UDPListener>,
  send_targets: HashMap<Token, mio_channel::Sender<Vec<u8>>>,
  message_receiver: MessageReceiver,

  // Adding readers
  receiver_add_reader: TokenReceiverPair<Reader>,
  receiver_remove_reader: TokenReceiverPair<GUID>,

  // Writers
  add_writer_receiver: TokenReceiverPair<Writer>,
  remove_writer_receiver: TokenReceiverPair<GUID>,
}

impl DPEventWrapper {
  pub fn new(
    udp_listeners: HashMap<Token, UDPListener>,
    send_targets: HashMap<Token, mio_channel::Sender<Vec<u8>>>,
    participant_guid_prefix: GuidPrefix,
    receiver_add_reader: TokenReceiverPair<Reader>,
    receiver_remove_reader: TokenReceiverPair<GUID>,
    add_writer_receiver: TokenReceiverPair<Writer>,
    remove_writer_receiver: TokenReceiverPair<GUID>,
  ) -> DPEventWrapper {
    let poll = Poll::new().expect("Unable to create new poll.");

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
        &receiver_add_reader.receiver,
        receiver_add_reader.token.clone(),
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register reader adder.");

    poll
      .register(
        &receiver_remove_reader.receiver,
        receiver_remove_reader.token.clone(),
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

    DPEventWrapper {
      poll,
      udp_listeners,
      send_targets,
      message_receiver: MessageReceiver::new(participant_guid_prefix),
      receiver_add_reader,
      receiver_remove_reader,
      add_writer_receiver,
      remove_writer_receiver,
    }
  }

  pub fn event_loop(mut ev_wrapper: DPEventWrapper) {
    let mut writers: HashMap<GUID, Writer> = HashMap::new();
    loop {
      let mut events = Events::with_capacity(1024);

      ev_wrapper
        .poll
        .poll(&mut events, None)
        .expect("Failed in waiting of poll.");
      for event in events.into_iter() {
        println!("Dp_eventwrapper poll received: {:?}", event); // for debugging!!!!!!

        if event.token() == STOP_POLL_TOKEN {
          // print for testing
          //println!("{}", ev_wrapper.message_receiver.available_readers.len());
          return;
        } else if DPEventWrapper::is_udp_traffic(&event) {
          ev_wrapper.handle_udp_traffic(&event);
        } else if DPEventWrapper::is_reader_action(&event) {
          ev_wrapper.handle_reader_action(&event);
        } else if DPEventWrapper::is_writer_action(&event) {
          ev_wrapper.handle_writer_action(&event, &mut writers);
        }
      }
    }
  }

  pub fn is_udp_traffic(event: &Event) -> bool {
    event.token() == DISCOVERY_SENDER_TOKEN || event.token() == USER_TRAFFIC_SENDER_TOKEN
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

  pub fn handle_udp_traffic(&mut self, event: &Event) {
    let listener = self.udp_listeners.get(&event.token());
    match listener {
      Some(l) => loop {
        let data = l.get_message();
        if data.is_empty() {
          return;
        }

        if event.token() == DISCOVERY_SENDER_TOKEN {
          self.message_receiver.handle_discovery_msg(data);
        } else if event.token() == USER_TRAFFIC_SENDER_TOKEN {
          self.message_receiver.handle_user_msg(data);
        }
      },
      None => return,
    };
  }

  pub fn handle_reader_action(&mut self, event: &Event) {
    match event.token() {
      ADD_READER_TOKEN => {
        let new_reader = self
          .receiver_add_reader
          .receiver
          .try_recv()
          .expect("Can't receive new reader");
        self.message_receiver.add_reader(new_reader);
      }
      REMOVE_READER_TOKEN => {
        let old_reader_guid = self.receiver_remove_reader.receiver.try_recv().unwrap();
        self.message_receiver.remove_reader(old_reader_guid);
      }
      _ => {}
    }
  }

  pub fn handle_writer_action(&mut self, event: &Event, writers: &mut HashMap<GUID, Writer>) {
    println!("dp_ew handle writer action with token {:?}", event.token());
    match event.token() {
      ADD_WRITER_TOKEN => {
        let new_writer = self
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
        writers.insert(new_writer.as_entity().guid, new_writer);
      }
      REMOVE_WRITER_TOKEN => {
        let writer = writers.remove(
          &self
            .remove_writer_receiver
            .receiver
            .try_recv()
            .expect("Failed to get writer reciver guid"),
        );
        match writer {
          Some(w) => {
            &self.poll.deregister(w.cache_change_receiver());
          }
          None => {}
        };
      }
      t => {
        let found_writer = writers.iter_mut().find(|p| p.1.get_entity_token() == t);

        match found_writer {
          Some((_guid, w)) => {
            let cache_change = w.cache_change_receiver().try_recv();
            println!("found RTPS writer with entity token {:?} ", t);

            match cache_change {
              Ok(cc) => {
                w.insert_to_history_cache(cc);
                w.send_all_unsend_messages();
              }
              _ => (),
            }
          }
          None => {}
        }
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
  use crate::dds::ddsdata::DDSData;
  use crate::structure::time::Timestamp;
  //use std::sync::mpsc;

  #[test]
  fn dpew_add_and_remove_readers() {
    // Adding readers
    let (sender_add_reader, receiver_add) = mio_channel::channel::<Reader>();
    let (sender_remove_reader, receiver_remove) = mio_channel::channel::<GUID>();

    let (_add_writer_sender, add_writer_receiver) = mio_channel::channel();
    let (_remove_writer_sender, remove_writer_receiver) = mio_channel::channel();

    let dp_event_wrapper = DPEventWrapper::new(
      HashMap::new(),
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
      let (send, _rec) = mio_channel::channel::<(DDSData, Timestamp)>();
      let new_reader = Reader::new(new_guid, send);

      reader_guids.push(new_reader.get_guid());
      println!("\nSent reader number {}: {:?}\n", i, new_reader);
      sender_add_reader.send(new_reader).unwrap();
      std::thread::sleep(Duration::new(0, 100));
    }

    println!("\npoistetaan toka\n");
    let some_guid = reader_guids[1];
    sender_remove_reader.send(some_guid).unwrap();
    std::thread::sleep(Duration::new(0, 100));

    println!("\nLopetustoken lähtee\n");
    sender_stop.send(0).unwrap();
    child.join().unwrap();
  }
}
