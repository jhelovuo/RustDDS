#[allow(unused_imports)]
use log::{debug,warn,error,trace, info};

use mio::net::UdpSocket;
use std::net::{SocketAddr, IpAddr};
use socket2::{Socket,Domain, Type, SockAddr, Protocol, };

#[cfg(test)] use std::net::Ipv4Addr;
use std::io;
use crate::structure::locator::{LocatorKind, Locator};
use crate::network::util::get_local_multicast_ip_addrs;

// We need one multicast sender socket per interface

#[derive(Debug)]
pub struct UDPSender {
  unicast_socket: UdpSocket,
  multicast_sockets: Vec<UdpSocket>,
}

impl UDPSender {
  pub fn new(sender_port: u16) -> io::Result<UDPSender> {
    let saddr: SocketAddr = SocketAddr::new("0.0.0.0".parse().unwrap(), sender_port);
    let unicast_socket: UdpSocket = UdpSocket::bind(&saddr)?;
    // We set multicasting loop on so that we can hear other DomainParticipant
    // instances running on the same host.
    unicast_socket.set_multicast_loop_v4(true)
      .unwrap_or_else(|e| { error!("Cannot set multicast loop on: {:?}",e); } );

    let mut multicast_sockets = Vec::with_capacity(1);
    for multicast_if_ipaddr in get_local_multicast_ip_addrs()? {
      let raw_socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP) )?;
      // beef: specify otput interface
      info!("UDPSender: Multicast sender on interface {:?}",multicast_if_ipaddr);
      match multicast_if_ipaddr {
        IpAddr::V4(a) => {
          raw_socket.set_multicast_if_v4(&a)?;
          raw_socket.bind( &SockAddr::from(SocketAddr::new(multicast_if_ipaddr, 0)) )?;
        }   
        IpAddr::V6(_a) => error!("UDPSender::new() not implemented for IpV6") , // TODO
      }
      
      let mc_socket = std::net::UdpSocket::from( raw_socket );
      mc_socket.set_multicast_loop_v4(true)
        .unwrap_or_else(|e| { error!("Cannot set multicast loop on: {:?}",e); } );
      multicast_sockets.push( UdpSocket::from_socket(mc_socket)? );
    } // end for

    let sender = UDPSender { unicast_socket, multicast_sockets, };
    info!("UDPSender::new() --> {:?}", sender);
    Ok(sender)
  }

  pub fn new_with_random_port() -> io::Result<UDPSender> {
    Self::new(0)
  }


  pub fn send_to_locator_list(&self, buffer: &[u8], ll: &[Locator]) {
    for loc in ll {
      self.send_to_locator(buffer,loc)
    }
  }

  fn send_to_udp_socket(&self, buffer: &[u8], socket: &UdpSocket, addr: &SocketAddr) {
    match socket.send_to(buffer, addr) {
      Ok(bytes_sent) =>
        if bytes_sent == buffer.len() { () // ok
        } else {
          error!("send_to_locator - send_to tried {} bytes, sent only {}",
              buffer.len(), bytes_sent);
        }
      Err(e) => {
        warn!("send_to_locator - send_to {} : {:?}", addr, e);
      }
    }    
  }

  pub fn send_to_locator(&self, buffer: &[u8], l: &Locator) {
      match l.kind {
        LocatorKind::LOCATOR_KIND_UDPv4 |
        LocatorKind::LOCATOR_KIND_UDPv6 => {
          let a = SocketAddr::from(l.to_socket_address());
          if a.ip().is_multicast() {
            for socket in self.multicast_sockets.iter() {
              self.send_to_udp_socket(buffer, &socket, &a);
            }
          } else {
            self.send_to_udp_socket(buffer, &self.unicast_socket, &a);
          }
        }
        LocatorKind::LOCATOR_KIND_INVALID |
        LocatorKind::LOCATOR_KIND_RESERVED =>
          error!("send_to_locator: Cannot send to {:?}",l.kind),
        _unknown_kind  =>
          // This is normal, as other implementations can define their own kinds.
          // We get those from Discovery.
          trace!("send_to_locator: Unknown LocatorKind: {:?}",l.kind),
      }
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

}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::network::udp_listener::*;
  use mio::Token;

  #[test]
  fn udps_single_send() {
    let listener = UDPListener::new_unicast(Token(0), "127.0.0.1", 10201).unwrap();
    let sender = UDPSender::new(11201).expect("failed to create UDPSender");

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
    let sender = UDPSender::new(11301).expect("failed to create UDPSender");

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
