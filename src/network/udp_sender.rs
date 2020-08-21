use mio::net::UdpSocket;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::io;
use crate::structure::locator::{LocatorKind, LocatorList};

#[derive(Debug)]
pub struct UDPSender {
  socket: UdpSocket,
}

fn create_socket_to_available_port() -> Option<UdpSocket> {
  for port in 1025..65535 {
    let saddr: SocketAddr = SocketAddr::new("0.0.0.0".parse().unwrap(), port);
    match UdpSocket::bind(&saddr) {
      Ok(l) => return Some(l),
      _ => {}
    }
  }
  None
}

impl UDPSender {
  pub fn new(sender_port: u16) -> UDPSender {
    let saddr: SocketAddr = SocketAddr::new("0.0.0.0".parse().unwrap(), sender_port);
    let socket: UdpSocket = UdpSocket::bind(&saddr).unwrap();

    UDPSender { socket: socket }
  }

  pub fn new_with_random_port() -> UDPSender {
    let socket: UdpSocket = create_socket_to_available_port().unwrap();
    UDPSender { socket: socket }
  }

  pub fn send_to_all(&self, buffer: &[u8], addresses: &Vec<SocketAddr>) {
    for address in addresses.iter() {
      match self.socket.send_to(buffer, address) {
        Ok(_) => (),
        _ => println!("Unable to send to {}", address),
      };
    }
  }

  pub fn send_to_locator_list(&self, buffer: &[u8], locators: &LocatorList) {
    for l in locators {
      if l.kind == LocatorKind::LOCATOR_KIND_UDPv4 || l.kind == LocatorKind::LOCATOR_KIND_UDPv6 {
        let a = SocketAddr::from(l.to_socket_address());
        match self.socket.send_to(buffer, &a) {
          Ok(_) => (println!("send udp message to socket {:?}", a)),
          _ => println!("Unable to send to {}", a),
        };
      }
    }
  }

  pub fn send_multicast(self, buffer: &[u8], address: Ipv4Addr, port: u16) -> io::Result<usize> {
    if address.is_multicast() {
      let address = SocketAddr::new(IpAddr::V4(address), port);
      return self.socket.send_to(buffer, &SocketAddr::from(address));
    }
    io::Result::Err(io::Error::new(
      io::ErrorKind::Other,
      "Not a multicast address",
    ))
  }
  pub fn send_ipv4_multicast(&self, buffer: &[u8], address: SocketAddr) -> io::Result<usize> {
    if address.ip().is_multicast() {
      return self.socket.send_to(buffer, &SocketAddr::from(address));
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
  use crate::network::udp_listener::*;
  use mio::Token;

  #[test]
  fn udps_single_send() {
    let listener = UDPListener::new(Token(0), "127.0.0.1", 10201);
    let sender = UDPSender::new(11201);

    let data: Vec<u8> = vec![0, 1, 2, 3, 4];

    let addrs = vec![SocketAddr::new("127.0.0.1".parse().unwrap(), 10201)];
    sender.send_to_all(&data, &addrs);

    let rec_data = listener.get_message();

    assert_eq!(rec_data.len(), 5);
    assert_eq!(rec_data, data);
  }

  #[test]
  fn udps_multi_send() {
    let listener_1 = UDPListener::new(Token(0), "127.0.0.1", 10301);
    let listener_2 = UDPListener::new(Token(1), "127.0.0.1", 10302);
    let sender = UDPSender::new(11301);

    let data: Vec<u8> = vec![5, 4, 3, 2, 1, 0];

    let addrs = vec![
      SocketAddr::new("127.0.0.1".parse().unwrap(), 10301),
      SocketAddr::new("127.0.0.1".parse().unwrap(), 10302),
    ];
    sender.send_to_all(&data, &addrs);

    let rec_data_1 = listener_1.get_message();
    let rec_data_2 = listener_2.get_message();

    assert_eq!(rec_data_1.len(), 6);
    assert_eq!(rec_data_1, data);
    assert_eq!(rec_data_2.len(), 6);
    assert_eq!(rec_data_2, data);
  }
}
