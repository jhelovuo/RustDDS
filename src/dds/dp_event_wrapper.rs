use mio::{Poll, Event, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;

use std::collections::HashMap;

use crate::dds::message_receiver::MessageReceiver;
use crate::dds::reader::Reader;
use crate::network::udp_listener::UDPListener;
use crate::network::constant::*;
use crate::structure::guid::{GuidPrefix, GUID};

pub struct DPEventWrapper {
  poll: Poll,
  udp_listeners: HashMap<Token, UDPListener>,
  send_targets: HashMap<Token, mio_channel::Sender<Vec<u8>>>,
  message_receiver: MessageReceiver,

  // Adding readers
  receiver_add_reader: TokenReceiverPair<Reader>,
  receiver_remove_reader: TokenReceiverPair<GUID>,
}

impl DPEventWrapper {
  pub fn new(
    udp_listeners: HashMap<Token, UDPListener>,
    send_targets: HashMap<Token, mio_channel::Sender<Vec<u8>>>,
    participant_guid_prefix: GuidPrefix,
    receiver_add_reader: TokenReceiverPair<Reader>,
    receiver_remove_reader: TokenReceiverPair<GUID>,
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

    DPEventWrapper {
      poll,
      udp_listeners,
      send_targets,
      message_receiver: MessageReceiver::new(participant_guid_prefix),
      receiver_add_reader,
      receiver_remove_reader,
    }
  }

  pub fn event_loop(mut ev_wrapper: DPEventWrapper) {
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
          .expect("Can't get new reader");
        self.message_receiver.add_reader(new_reader);
      }
      REMOVE_READER_TOKEN => {
        let old_reader_guid = self.receiver_remove_reader.receiver.try_recv().unwrap();
        self.message_receiver.remove_reader(old_reader_guid);
      }
      _ => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::thread;
  use std::time::Duration;
  use mio::{Ready, Registration, Poll, PollOpt, Token, SetReadiness};
  use crate::structure::entity::Entity;
  //use std::sync::mpsc;

  #[test]
  fn dpew_add_and_remove_readers() {
    // Adding readers
    let (sender_add_reader, receiver_add) = mio_channel::channel::<Reader>();
    let (sender_remove_reader, receiver_remove) = mio_channel::channel::<GUID>();

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

      let mut new_reader = Reader::new(new_guid);

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
