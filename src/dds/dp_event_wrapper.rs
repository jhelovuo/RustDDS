use mio::{Poll, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;

use std::collections::HashMap;

use crate::network::udp_listener::UDPListener;
use crate::network::constant::*;

pub struct DPEventWrapper {
  poll: Poll,
  events: Events,
  listeners: HashMap<Token, UDPListener>,
  available_token: usize,
  channel_receivers: HashMap<Token, mio_channel::Receiver<UDPListener>>,
}

impl DPEventWrapper {
  pub fn new(
    channel_senders: &mut HashMap<Token, mio_channel::Sender<UDPListener>>,
  ) -> DPEventWrapper {
    let poll = Poll::new().expect("Unable to create new poll.");

    let (rudpl_sender, rudpl_receiver) = mio_channel::channel();
    poll
      .register(
        &rudpl_receiver,
        ADD_UDP_LISTENER_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register add_udp_listener token.");

    channel_senders.insert(ADD_UDP_LISTENER_TOKEN, rudpl_sender);

    let mut channel_receivers = HashMap::new();
    channel_receivers.insert(ADD_UDP_LISTENER_TOKEN, rudpl_receiver);

    DPEventWrapper {
      poll: poll,
      events: Events::with_capacity(1024),
      listeners: HashMap::new(),
      available_token: START_FREE_TOKENS.0,
      channel_receivers: channel_receivers,
    }
  }

  fn get_available_token(&mut self) -> usize {
    let token = self.available_token;
    self.available_token = self.available_token + 1;
    token
  }

  fn register_udp_listener(&mut self, mut listener: UDPListener) {
    let token = Token(self.get_available_token());
    listener.set_token(token);
    let mio_socket = listener.mio_socket();

    self
      .poll
      .register(mio_socket, token, Ready::readable(), PollOpt::edge())
      .expect("Failed to register poll for udp listener.");
    self.listeners.insert(token, listener);
  }

  pub fn event_loop(mut ev_wrapper: DPEventWrapper) {
    loop {
      ev_wrapper
        .poll
        .poll(&mut ev_wrapper.events, None)
        .expect("Failed in waiting of poll.");

      for event in &ev_wrapper.events {
        if event.token() == STOP_POLL_TOKEN {
          return;
        } else if event.token() == ADD_UDP_LISTENER_TOKEN {

          // ev_wrapper.register_udp_listener()
        }

        let mut listener = ev_wrapper.listeners.get(&event.token());
        let mut datas: Vec<Vec<u8>> = vec![];
        match listener {
          Some(l) => {
            while let data = l.get_message() {
              if data.is_empty() {
                break;
              }
              datas.push(data);
            }
          }
          None => continue,
        }
      }
    }
  }
}
