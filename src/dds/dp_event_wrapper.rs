use mio::{Poll, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;

use std::collections::{HashMap, HashSet};

use crate::network::udp_listener::UDPListener;
use crate::network::udp_sender::UDPSender;
use crate::network::constant::*;

pub struct DPEventWrapper {
  poll: Poll,
  add_listener_channel: mio_channel::Receiver<(Token, UDPListener)>,
  udp_listeners: HashMap<Token, UDPListener>,
  send_targets: HashMap<Token, mio_channel::Sender<(Token, Vec<Vec<u8>>)>>,
}

impl DPEventWrapper {
  pub fn new(
    add_listener_channel: mio_channel::Receiver<(Token, UDPListener)>,
    udp_listeners: HashMap<Token, UDPListener>,
    send_targets: HashMap<Token, mio_channel::Sender<(Token, Vec<Vec<u8>>)>>,
  ) -> DPEventWrapper {
    let poll = Poll::new().expect("Unable to create new poll.");

    poll
      .register(
        &add_listener_channel,
        ADD_UDP_LISTENER_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register add_udp_listener token.");

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
      add_listener_channel: add_listener_channel,
      udp_listeners: udp_listeners,
      send_targets: send_targets,
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
        println!("Event loop: token {:?}", event.token());
        if event.token() == STOP_POLL_TOKEN {
          return;
        } else if event.token() == ADD_UDP_LISTENER_TOKEN {
          let target = &mut ev_wrapper.add_listener_channel;
          match target.try_recv() {
            Ok((token, mut l)) => {
              if !ev_wrapper.udp_listeners.contains_key(&token) {
                ev_wrapper
                  .poll
                  .register(
                    l.mio_socket(),
                    token.clone(),
                    Ready::readable(),
                    PollOpt::edge(),
                  )
                  .expect("Unable to register new upd listener.");
                ev_wrapper.udp_listeners.insert(token.clone(), l);
              }
            }
            _ => continue,
          };
        } else {
          let listener = ev_wrapper.udp_listeners.get(&event.token());
          let mut datas: Vec<Vec<u8>> = vec![];
          match listener {
            Some(l) => loop {
              let data = l.get_message();
              if data.is_empty() {
                break;
              }
              datas.push(data);
            },
            None => continue,
          };

          let send_target = ev_wrapper.send_targets.get(&event.token());
          match send_target {
            Some(t) => {
              t.send((event.token(), datas))
                .expect("Unable to send data to target");
            }
            None => continue,
          };
        }
      }
    }
  }
}
