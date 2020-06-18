use mio::{Poll, Events, Token, Interest};
use std::thread;
use std::collections::HashMap;

use crate::network::udp_listener::UDPListener;
use crate::network::constant::STOP_POLL_TOKEN;

struct DomainParticipant {
  poll: Poll,
  events: Events,
  listeners: HashMap<Token, UDPListener>,
}

impl DomainParticipant {
  pub fn new() -> DomainParticipant {
    DomainParticipant {
      poll: Poll::new().expect("Unable to create new poll."),
      events: Events::with_capacity(1024),
      listeners: HashMap::new(),
    }
  }

  pub fn register_udp_listener(self, listener: UDPListener) {
    self.listeners.push(listener);
    poll
      .registy()
      .register(listener.mio_socket(), listener.token(), Interest::READABLE)
  }

  pub fn event_loop(&mut self) {
    loop {
      self
        .poll
        .poll(&mut self.events, None)
        .expect("Failed in waiting of poll.");

      for event in &events {
        if event.token() == STOP_POLL_TOKEN {
          return;
        }

        let listener = self.listeners.get(event.token());
        let datas: Vec<Vec<u8>> = vec![];
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
