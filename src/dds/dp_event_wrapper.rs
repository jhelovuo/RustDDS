use mio::{Poll, Event, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;

use std::collections::HashMap;

use crate::dds::message_receiver::MessageReceiver;
use crate::network::udp_listener::UDPListener;
use crate::network::constant::*;
use crate::structure::guid::{GuidPrefix};

pub struct DPEventWrapper {
  poll: Poll,
  udp_listeners: HashMap<Token, UDPListener>,
  send_targets: HashMap<Token, mio_channel::Sender<Vec<u8>>>,
  message_receiver: MessageReceiver,
}

impl DPEventWrapper {
  pub fn new(
    udp_listeners: HashMap<Token, UDPListener>,
    send_targets: HashMap<Token, mio_channel::Sender<Vec<u8>>>,
    participant_guid_prefix: GuidPrefix,
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

    DPEventWrapper {
      poll: poll,
      udp_listeners: udp_listeners,
      send_targets: send_targets,
      message_receiver: MessageReceiver::new(participant_guid_prefix),
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
        if event.token() == STOP_POLL_TOKEN {
          return;
        } else if DPEventWrapper::is_udp_traffic(&event) {
          ev_wrapper.handle_udp_traffic(&event);
        }
      }
    }
  }

  pub fn is_udp_traffic(event: &Event) -> bool {
    event.token() == DISCOVERY_SENDER_TOKEN || event.token() == USER_TRAFFIC_SENDER_TOKEN
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
            //self.message_receiver.handle_discovery_msg(data);
          } else if event.token() == USER_TRAFFIC_SENDER_TOKEN {
            self.message_receiver.handle_user_msg(data);
          }
        },
        None => return,
      };
  }
}
