use std::net::{Ipv4Addr, SocketAddr};
use std::io;

use mio::Token;
use log::{debug, error};
use mio::net::UdpSocket;
//use std::net::UdpSocket as StdUdpSocket;

use socket2::{Socket,Domain, Type, SockAddr, Protocol, };

// 64 kB buffer size
// TODO: This is horribly inefficient. We allocate 64 kB for even
// the smallest UDP packets. However, it this is not easy to scale down, because it is
// not easy to know the size of incoming UDP packets other than actually receiving them.
const BUFFER_SIZE: usize = 64 * 1024;

/// Listens to messages coming to specified host port combination.
/// Only messages from added listen addressed are read when get_all_messages is called.
#[derive(Debug)]
pub struct UDPListener {
  socket: UdpSocket,
  token: Token,
}

// TODO: Remove panics from this function. Convert return value to Result.
impl UDPListener {

  // TODO: Why is is this function even necessary? Doesn't try_bind() do just the same?
  pub fn new(token: Token, host: &str, port: u16) -> UDPListener {
    let raw_socket = Socket::new(Domain::ipv4(), Type::dgram(), Some(Protocol::udp()) ).unwrap();

    // We set ReuseAddr so that other DomainParticipants on this host can
    // bind to the same multicast address and port.
    // To have an effect on bind, this must be done before bind call, so must be done
    // below Rust std::net::UdpSocket level.
    raw_socket.set_reuse_address(true) 
      .map_err(|e| error!("Unable set SO_REUSEADDR option on socket {:?}", e))
      .unwrap();

    let address = SocketAddr::new(host.parse().unwrap(), port);
    let err_msg = format!("new - Unable to bind address {}", address.to_string());

    raw_socket.bind( &SockAddr::from(address) )
      .expect(&err_msg);

    let std_socket = raw_socket.into_udp_socket();
    std_socket
      .set_nonblocking(true)
      .expect("Failed to set std socket to non blocking.");

    let mio_socket = UdpSocket::from_socket(std_socket)
                      .expect("Unable to create mio socket");
    debug!("UDPListener::new with address {:?}", mio_socket.local_addr());

    UDPListener { socket: mio_socket, token }
  }

  // TODO: convert return value from Option to Result
  pub fn try_bind(token: Token, host: &str, port: u16) -> Option<UDPListener> {
    let host = match host.parse() {
      Ok(h) => h,
      _ => return None,
    };

    let address = SocketAddr::new(host, port);
    let raw_socket = Socket::new(Domain::ipv4(), Type::dgram(), Some(Protocol::udp()) ).unwrap();

    // We set ReuseAddr so that other DomainParticipants on this host can
    // bind to the same multicast address and port.
    // To have an effect on bind, this must be done before bind call, so must be done
    // below Rust std::net::UdpSocket level.
    raw_socket.set_reuse_address(true) 
      .map_err(|e| error!("Unable set SO_REUSEADDR option on socket {:?}", e))
      .unwrap();

    match raw_socket.bind( &SockAddr::from(address) ) {
      Err(e) => {
        error!("try bind - cannot bind socket: {:?}",e);
        return None
      }
      _ => (), // Ok
    }
    let std_socket = raw_socket.into_udp_socket();

    match std_socket.set_nonblocking(true) {
      Ok(_) => (),
      Err(e) => {
        error!("Failed to set std socket to non blocking. {:?}", e);
        return None
      }
    };

    let socket = match UdpSocket::from_socket(std_socket) {
      Ok(s) => s,
      Err(e) => {
        error!("Failed to create mio socket. {:?}", e);
        return None
      }
    };
    debug!("UDPListener::try_bind with address {:?}", socket.local_addr());

    Some(UDPListener { socket, token })
  }

  pub fn get_token(&self) -> Token {
    self.token
  }

  pub fn mio_socket(&mut self) -> &mut UdpSocket {
    &mut self.socket
  }

  pub fn port(&self) -> u16 {
    match self.socket.local_addr() {
      Ok(add) => add.port(),
      _ => 0,
    }
  }

  /// Returns all messages that have come from listen_addresses.
  /// Converts/prunes individual results to Vec
  pub fn get_message(&self) -> Vec<u8> {
    let mut message: Vec<u8> = vec![];
    let mut buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    //self.mio_socket().recv_from(buf)
    match self.socket.recv(&mut buf) {
      Ok(nbytes) => {
        message = buf[..nbytes].to_vec();
        return message;
      }
      Err(e) => {
        debug!("UDPListener::get_message failed: {:?}", e);
        ()
      }
    };
    message
  }

  pub fn get_messages(&self) -> Vec<Vec<u8>> {
    let mut datas = vec![];
    let mut buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

    while let Ok(nbytes) = self.socket.recv(&mut buf) {
      datas.push(buf[..nbytes].to_vec());
    }

    datas
  }

  pub fn join_multicast(&self, address: &Ipv4Addr) -> io::Result<()> {
    if address.is_multicast() {
      return self
        .socket
        .join_multicast_v4(address, &Ipv4Addr::UNSPECIFIED);
    }
    io::Result::Err(io::Error::new(
      io::ErrorKind::Other,
      "Not a multicast address",
    ))
  }

  pub fn leave_multicast(&self, address: &Ipv4Addr) -> io::Result<()> {
    if address.is_multicast() {
      return self
        .socket
        .leave_multicast_v4(address, &Ipv4Addr::UNSPECIFIED);
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

  //use std::os::unix::io::AsRawFd;
  //use nix::sys::socket::setsockopt;
  //use nix::sys::socket::sockopt::IpMulticastLoop;

  use std::{thread, time};

  #[test]
  fn udpl_single_address() {
    let listener = UDPListener::new(Token(0), "127.0.0.1", 10001);
    let sender = UDPSender::new_with_random_port();

    let data: Vec<u8> = vec![0, 1, 2, 3, 4];

    let addrs = vec![SocketAddr::new("127.0.0.1".parse().unwrap(), 10001)];
    sender.send_to_all(&data, &addrs);

    let rec_data = listener.get_message();

    assert_eq!(rec_data.len(), 5);
    assert_eq!(rec_data, data);
  }

  #[test]
  fn udpl_multicast_address() {
    let listener = UDPListener::new(Token(0), "0.0.0.0", 10002);
    let sender = UDPSender::new_with_random_port();

    //setsockopt(sender.socket.as_raw_fd(), IpMulticastLoop, &true)
    //  .expect("Unable set IpMulticastLoop option on socket");

    let data: Vec<u8> = vec![2, 4, 6];

    listener
      .join_multicast(&Ipv4Addr::new(239, 255, 0, 1))
      .expect("Failed to join multicast.");

    sender
      .send_multicast(&data, Ipv4Addr::new(239, 255, 0, 1), 10002)
      .expect("Failed to send multicast");

    thread::sleep(time::Duration::from_secs(1));

    let rec_data = listener.get_message();

    listener
      .leave_multicast(&Ipv4Addr::new(239, 255, 0, 1))
      .unwrap();

    assert_eq!(rec_data.len(), 3);
    assert_eq!(rec_data, data);
  }
}
