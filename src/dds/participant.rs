use mio::Token;
use mio_extras::channel as mio_channel;

use std::thread;
use std::collections::HashMap;

use crate::network::udp_listener::UDPListener;
use crate::dds::dp_event_wrapper::DPEventWrapper;

pub struct DomainParticipant {
  senders: HashMap<Token, mio_channel::Sender<UDPListener>>,
}

impl DomainParticipant {
  pub fn new() -> DomainParticipant {
    let mut senders = HashMap::new();
    let ev_wrapper = DPEventWrapper::new(&mut senders);
    thread::spawn(move || DPEventWrapper::event_loop(ev_wrapper));
    DomainParticipant { senders: senders }
  }
}
