use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr};
use std::io;

use mio::Token;
use mio::net::UdpSocket;
use std::net::UdpSocket as StdUdpSocket;

// 64 kB buffer size
const BUFFER_SIZE: usize = 64 * 1024;

/// Listens to messages coming to specified host port combination.
/// Only messages from added listen addressed are read when get_all_messages is called.
#[derive(Debug)]
pub struct UDPListener {
  socket: UdpSocket,
  token: Token,
}

impl UDPListener {
  pub fn new(token: Token, host: &str, port: u16) -> UDPListener {
    let address = SocketAddr::new(host.parse().unwrap(), port);
    let err_msg = format!("Unable to bind address {}", address.to_string());
    let std_socket = StdUdpSocket::bind(address).expect(&err_msg);
    std_socket
      .set_nonblocking(true)
      .expect("Failed to set std socket to non blocking.");
    let socket = UdpSocket::from_socket(std_socket).expect("Unable to create mio socket");

    UDPListener {
      socket: socket,
      token: Token(1),
    }
  }

  pub fn get_token(&self) -> Token {
    self.token
  }

  pub fn mio_socket(&mut self) -> &mut UdpSocket {
    &mut self.socket
  }

  /// Returns all messages that have come from listen_addresses.
  /// Converts/prunes individual results to Vec
  pub fn get_message(&self) -> Vec<u8> {
    let mut message: Vec<u8> = vec![];
    let mut buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    if let Ok((nbytes, address)) = self.socket.recv_from(&mut buf) {
      message = buf[..nbytes].to_vec();
    }
    message
  }

  pub fn join_multicast(&self, address: &Ipv4Addr) -> io::Result<()> {
    if address.is_multicast() {
      let inter = Ipv4Addr::new(0, 0, 0, 0);
      return self.socket.join_multicast_v4(address, &inter);
    }
    io::Result::Err(io::Error::new(
      io::ErrorKind::Other,
      "Not a multicast address",
    ))
  }

  pub fn leave_multicast(&self, address: &Ipv4Addr) -> io::Result<()> {
    if address.is_multicast() {
      let inter = Ipv4Addr::new(0, 0, 0, 0);
      return self.socket.leave_multicast_v4(address, &inter);
    }
    io::Result::Err(io::Error::new(
      io::ErrorKind::Other,
      "Not a multicast address",
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::network::udp_sender::*;

  #[test]
  fn udpl_single_address() {
    let mut listener = UDPListener::new(Token(0), "127.0.0.1", 10001);
    let mut sender = UDPSender::new(11001);

    let data: Vec<u8> = vec![0, 1, 2, 3, 4];

    let addrs = vec![SocketAddr::new("127.0.0.1".parse().unwrap(), 10001)];
    sender.send_to_all(&data, &addrs);

    let rec_data = listener.get_message();

    assert_eq!(rec_data.len(), 5);
    assert_eq!(rec_data, data);
  }

  // TODO: there is something wrong with this test (possibly inability actually send or receive multicast)
  // #[test]
  fn udpl_multicast_address() {
    let mut listener = UDPListener::new(Token(0), "127.0.0.1", 10002);
    let mut sender = UDPSender::new(11002);

    let data: Vec<u8> = vec![2, 4, 6];

    // still need to use the same port
    let ipmcaddr = Ipv4Addr::new(240, 0, 0, 12);
    let mcaddr = vec![SocketAddr::new("224.0.0.12".parse().unwrap(), 10002)];
    listener.join_multicast(&ipmcaddr);

    sender.send_to_all(&data, &mcaddr);

    let rec_data = listener.get_message();

    listener.leave_multicast(&ipmcaddr);

    assert_eq!(rec_data.len(), 3);
    assert_eq!(rec_data, data);
  }
}
