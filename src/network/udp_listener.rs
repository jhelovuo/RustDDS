use std::net::{Ipv4Addr, SocketAddr};
use std::io;

use mio::Token;
use mio::net::UdpSocket;

use log::{debug, error, trace};

use socket2::{Socket,Domain, Type, SockAddr, Protocol, };

use bytes::{Bytes,BytesMut};

const MAX_MESSAGE_SIZE : usize = 64 * 1024; // This is max we can get from UDP.
const MESSAGE_BUFFER_ALLOCATION_CHUNK : usize = 256 * 1024; // must be >= MAX_MESSAGE_SIZE

/// Listens to messages coming to specified host port combination.
/// Only messages from added listen addressed are read when get_all_messages is called.
#[derive(Debug)]
pub struct UDPListener {
  socket: UdpSocket,
  token: Token,
  receive_buffer: BytesMut,
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

    UDPListener { socket: mio_socket, token,
      receive_buffer: BytesMut::with_capacity(MESSAGE_BUFFER_ALLOCATION_CHUNK),
    }
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

    Some(UDPListener { socket, token ,
      receive_buffer: BytesMut::with_capacity(MESSAGE_BUFFER_ALLOCATION_CHUNK)
    })
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

  // TODO: remove this. It is used only for tests.
  // We cannot read a single packet only, because we use edge-triggered polls.
  #[cfg(test)]
  pub fn get_message(&self) -> Vec<u8> {

    let mut message: Vec<u8> = vec![];
    let mut buf: [u8; MAX_MESSAGE_SIZE] = [0; MAX_MESSAGE_SIZE];
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

  fn ensure_receive_buffer_capacity(&mut self) {
    if self.receive_buffer.capacity() < MAX_MESSAGE_SIZE {
      self.receive_buffer = BytesMut::with_capacity(MESSAGE_BUFFER_ALLOCATION_CHUNK);
      debug!("ensure_receive_buffer_capacity - reallocated receive_buffer");
    }
    unsafe {
      // This is safe, because we just checked that there is enough capacity.
      // We do not read undefined data, because next the recv call in get_messages() 
      // will overwrite this space and truncate the rest away.
      self.receive_buffer.set_len(MAX_MESSAGE_SIZE)
    }
    trace!("ensure_receive_buffer_capacity - {} bytes left",self.receive_buffer.capacity());
  }

  /// Get all messages waiting in the socket.
  pub fn get_messages(&mut self) -> Vec<Bytes> {
    // This code may seem slighlty non-sensical, if you do not know
    // how BytesMut works.
    let mut messages = Vec::with_capacity(4); // just a guess, should cover most cases
    self.ensure_receive_buffer_capacity();
    while let Ok(nbytes) = self.socket.recv(&mut self.receive_buffer) {
      self.receive_buffer.truncate(nbytes);
      // Now append some extra data to align the buffer end, so the next piece will
      // be aligned also. This assumes that the initial buffer was aligned to begin with.
      while self.receive_buffer.len() % 4 != 0 {
        self.receive_buffer.extend_from_slice(&[0xCC]); // add some funny padding bytes
        // Funny value encourages fast crash in case these bytes are ever accessed,
        // as they should not.
      }
      let mut message = self.receive_buffer.split_to(self.receive_buffer.len());
      self.ensure_receive_buffer_capacity();
      message.truncate(nbytes); // discard (hide) padding
      messages.push( Bytes::from(message) ); // freeze and push
    }
    messages
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
