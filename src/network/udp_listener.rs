use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use mio::Token;
use mio::net::UdpSocket;
use std::net::UdpSocket as StdUdpSocket;

// 4 MB buffer size
// const BUFFER_SIZE: usize = 4294967296;
const BUFFER_SIZE: usize = 64 * 1024;

static available_token: AtomicUsize = AtomicUsize::new(1);

/// Listens to messages coming to specified host port combination.
/// Only messages from added listen addressed are read when get_all_messages is called.
#[derive(Debug)]
pub struct UDPListener {
  socket: UdpSocket,
  listen_addresses: HashSet<SocketAddr>,
  token: Token,
}

impl UDPListener {
  pub fn new(host: &str, port: u16) -> UDPListener {
    let address = SocketAddr::new(host.parse().unwrap(), port);
    let err_msg = format!("Unable to bind address {}", address.to_string());
    let std_socket = StdUdpSocket::bind(address).expect(&err_msg);
    std_socket
      .set_nonblocking(true)
      .expect("Failed to set std socket to non blocking.");
    let socket = UdpSocket::from_socket(std_socket).expect("Unable to create mio socket");

    UDPListener {
      socket: socket,
      listen_addresses: HashSet::new(),
      token: Token(UDPListener::get_available_token()),
    }
  }

  pub fn mio_socket(&mut self) -> &mut UdpSocket {
    &mut self.socket
  }

  fn get_available_token() -> usize {
    let token = available_token.load(Ordering::Relaxed);
    available_token.store(token + 1, Ordering::Relaxed);
    token
  }

  pub fn token(&self) -> &Token {
    &self.token
  }

  pub fn add_listen_address(&mut self, host: &str, port: u16) {
    let address = SocketAddr::new(host.parse().unwrap(), port);
    self.listen_addresses.insert(address);
  }

  pub fn remove_listen_address(&mut self, host: &str, port: u16) {
    let address = SocketAddr::new(host.parse().unwrap(), port);
    self.listen_addresses.remove(&address);
  }

  /// Returns all messages that have come from listen_addresses.
  /// Converts/prunes individual results to Vec
  pub fn get_message(&mut self) -> Vec<u8> {
    let mut message: Vec<u8> = vec![];
    let mut buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    if let Ok((nbytes, address)) = self.socket.recv_from(&mut buf) {
      if self.listen_addresses.contains(&address) {
        message = buf[..nbytes].to_vec();
      }
    }
    message
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::network::udp_sender::*;

  #[test]
  fn udpl_single_address() {
    let mut listener = UDPListener::new("127.0.0.1", 10001);
    let mut sender = UDPSender::new(11001);

    let data: Vec<u8> = vec![0, 1, 2, 3, 4];

    listener.add_listen_address("127.0.0.1", 11001);

    sender.add_send_address("127.0.0.1", 10001);
    sender.send_to_all(&data);

    let rec_data = listener.get_message();

    assert_eq!(rec_data.len(), 5);
    assert_eq!(rec_data, data);
  }

  #[test]
  fn udpl_multi_address() {
    let mut listener = UDPListener::new("127.0.0.1", 10101);
    let mut sender_1 = UDPSender::new(11101);
    let mut sender_2 = UDPSender::new(11102);

    let data_1: Vec<u8> = vec![0, 1, 2, 3, 4];

    listener.add_listen_address("127.0.0.1", 11101);
    listener.add_listen_address("127.0.0.1", 11102);

    sender_1.add_send_address("127.0.0.1", 10101);
    sender_1.send_to_all(&data_1);

    let data_2: Vec<u8> = vec![5, 4, 3, 2, 1, 0];
    sender_2.add_send_address("127.0.0.1", 10101);
    sender_2.send_to_all(&data_2);

    let rec_data = listener.get_message();
    assert_eq!(rec_data.len(), 5);
    assert_eq!(rec_data, data_1);

    let rec_data = listener.get_message();
    assert_eq!(rec_data.len(), 6);
    assert_eq!(rec_data, data_2);
  }
}
