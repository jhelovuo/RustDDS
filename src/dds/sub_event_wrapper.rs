use mio::{Poll, Event, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;

use std::collections::HashMap;

use crate::dds::reader::Reader;
use crate::dds::pubsub::{Subscriber, DataReader};
use crate::network::udp_listener::UDPListener;
use crate::network::constant::*;
use crate::structure::guid::{GuidPrefix, GUID};
use std::sync::{Arc, Mutex};

pub struct SubEventWrapper{
  poll: Poll,
  subscribers: Arc<Mutex<Vec<Subscriber>>>,

  receiver_add_datareader: TokenReceiverPair<DataReader>,
  receiver_remove_datareader: TokenReceiverPair<GUID>,
}

impl SubEventWrapper {
  pub fn new(
    subscribers: Arc<Mutex<Vec<Subscriber>>>,
    receiver_add_datareader: TokenReceiverPair<DataReader>,
    receiver_remove_datareader: TokenReceiverPair<GUID>,
  ) -> Self {
    let poll = Poll::new().expect("Unable to create new poll.");

    poll.register(
      &receiver_add_datareader.receiver,
      receiver_add_datareader.token.clone(),
      Ready::readable(),
      PollOpt::edge(),
    ).expect("Failed to register datareader adder.");

    poll.register(
      &receiver_remove_datareader.receiver,
      receiver_remove_datareader.token.clone(),
      Ready::readable(),
      PollOpt::edge(),
    ).expect("Failed to register datareader remover.");

    Self{
      poll,
      subscribers,
      receiver_add_datareader,
      receiver_remove_datareader,
    }
  }

  pub fn event_loop(mut sub_ev_wrapper: SubEventWrapper) {
    loop {
      let mut events = Events::with_capacity(1024);

      sub_ev_wrapper.poll.poll(
        &mut events, None
      ).expect("Subscriber failed in polling");

      for event in events.into_iter() {
        println!("Subscriber poll received: {:?}", event); // for debugging!!!!!!

        match event.token() {
          STOP_POLL_TOKEN => return,
          READER_CHANGE_TOKEN => {
            println!("Got notification of reader's new change!!!");
          },
          ADD_DATAREADER_TOKEN => {},
          REMOVE_DATAREADER_TOKEN => {},
          _ => {},
        }
      }
    }
  }
}