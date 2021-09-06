#[allow(unused_imports)]
use log::{debug,warn,error,trace};

use mio::net::UdpSocket;
use std::net::{SocketAddr};

#[cfg(test)] use std::net::{IpAddr, Ipv4Addr};
#[cfg(test)] use std::io;
use crate::structure::locator::{LocatorKind, LocatorList, Locator};

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
    // We set multicasting loop on so that we can hear other DomainParticipant
    // instances running on the same host.
    socket.set_multicast_loop_v4(true)
      .unwrap_or_else(|e| { error!("Cannot set multicast loop on: {:?}",e); } );
    UDPSender { socket: socket }
  }

  pub fn new_with_random_port() -> UDPSender {
    let socket: UdpSocket = create_socket_to_available_port().unwrap();
    UDPSender { socket: socket }
  }

  #[cfg(test)]
  pub fn send_to_all(&self, buffer: &[u8], addresses: &Vec<SocketAddr>) {
    for address in addresses.iter() {
      match self.socket.send_to(buffer, address) {
        Ok(_) => (),
        _ => debug!("Unable to send to {}", address),
      };
    }
  }

  pub fn send_to_locator_list(&self, buffer: &[u8], ll: &LocatorList) {
    for loc in ll.iter() {
      self.send_to_locator(buffer,loc)
    }
  }

  pub fn send_to_locator(&self, buffer: &[u8], l: &Locator) {
      match l.kind {
        LocatorKind::LOCATOR_KIND_UDPv4 |
        LocatorKind::LOCATOR_KIND_UDPv6 => {
          let a = SocketAddr::from(l.to_socket_address());
          match self.socket.send_to(buffer, &a) {
            Ok(bytes_sent) =>
              if bytes_sent == buffer.len() { () // ok
              } else {
                error!("send_to_locator - send_to tried {} bytes, sent only {}",
                    buffer.len(), bytes_sent);
              }
            Err(e) => {
              warn!("send_to_locator - send_to {} : {:?}", a, e);
            }
          }
        }
        LocatorKind::LOCATOR_KIND_INVALID |
        LocatorKind::LOCATOR_KIND_RESERVED =>
          error!("send_to_locator: Cannot send to {:?}",l.kind),
        _unknown_kind  =>
          // This is normal, as implementations can define their own kinds.
          trace!("send_to_locator: Unknown LocatorKind: {:?}",l.kind),
      }
  }

  #[cfg(test)]
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

  // pub fn send_ipv4_multicast(&self, buffer: &[u8], address: SocketAddr) -> io::Result<usize> {
  //   if address.ip().is_multicast() {
  //     return self.socket.send_to(buffer, &address);
  //   }
  //   io::Result::Err(io::Error::new(
  //     io::ErrorKind::Other,
  //     "Not a multicast address",
  //   ))
  // }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::network::udp_listener::*;
  use mio::Token;

  #[test]
  fn udps_single_send() {
    let listener = UDPListener::new_unicast(Token(0), "127.0.0.1", 10201).unwrap();
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
    let listener_1 = UDPListener::new_unicast(Token(0), "127.0.0.1", 10301).unwrap();
    let listener_2 = UDPListener::new_unicast(Token(1), "127.0.0.1", 10302).unwrap();
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
