use mio::{Poll, Events, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;

use std::collections::HashMap;

use crate::network::udp_listener::UDPListener;
use crate::network::udp_sender::UDPSender;
use crate::network::constant::*;

pub struct DPEventWrapper {
  poll: Poll,
  listeners: HashMap<Token, UDPListener>,
  available_token: usize,
  reader_receivers: HashMap<Token, mio_channel::Receiver<UDPListener>>,
  writer_receivers: HashMap<Token, mio_channel::Receiver<UDPSender>>,
}

impl DPEventWrapper {
  pub fn new(
    reader_senders: &mut HashMap<Token, mio_channel::Sender<UDPListener>>,
    writer_senders: &mut HashMap<Token, mio_channel::Sender<UDPSender>>,
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

    let (wudpl_sender, wudpl_receiver) = mio_channel::channel();
    poll
      .register(
        &wudpl_receiver,
        ADD_UDP_SENDER_TOKEN,
        Ready::writable(),
        PollOpt::edge(),
      )
      .expect("Failed to register add_udp_sender token.");

    reader_senders.insert(ADD_UDP_LISTENER_TOKEN, rudpl_sender);
    writer_senders.insert(ADD_UDP_SENDER_TOKEN, wudpl_sender);

    let mut reader_receivers = HashMap::new();
    reader_receivers.insert(ADD_UDP_LISTENER_TOKEN, rudpl_receiver);

    let mut writer_receivers = HashMap::new();
    writer_receivers.insert(ADD_UDP_SENDER_TOKEN, wudpl_receiver);

    DPEventWrapper {
      poll: poll,
      listeners: HashMap::new(),
      available_token: START_FREE_TOKENS.0,
      reader_receivers: reader_receivers,
      writer_receivers: writer_receivers,
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

  pub fn event_loop(ev_wrapper: DPEventWrapper) {
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
          // TODO: Listener and Sender should probably be registered with same token.
        } else if event.token() == ADD_UDP_SENDER_TOKEN {
          // TODO: Listener and Sender should probably be registered with same token.
        }

        let listener = ev_wrapper.listeners.get(&event.token());
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
        }
        // TODO: do something with the data
      }
    }
  }
}
