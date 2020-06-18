use mio::net::UdpSocket;
use std::net::SocketAddr;
use std::collections::HashSet;

pub struct UDPSender {
  sender_port: u16,
  send_addresses: HashSet<SocketAddr>,
}

impl UDPSender {
  pub fn new(sender_port: u16) -> UDPSender {
    UDPSender {
      sender_port: sender_port,
      send_addresses: HashSet::new(),
    }
  }

  pub fn add_send_address(&mut self, host: &str, port: u16) {
    let address = SocketAddr::new(host.parse().unwrap(), port);
    self.send_addresses.insert(address);
  }

  pub fn remove_send_address(&mut self, host: &str, port: u16) {
    let address = SocketAddr::new(host.parse().unwrap(), port);
    self.send_addresses.remove(&address);
  }

  pub fn send_to_all(self, buffer: &[u8]) {
    let saddr: SocketAddr = SocketAddr::new("0.0.0.0".parse().unwrap(), self.sender_port);
    let socket: UdpSocket = UdpSocket::bind(&saddr).unwrap();

    for &address in self.send_addresses.iter() {
      match socket.send_to(buffer, &address) {
        Ok(_) => (),
        _ => println!("Unable to send to {}", address),
      };
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::network::udp_listener::*;

  #[test]
  fn udps_single_send() {
    let mut listener = UDPListener::new("127.0.0.1", 10201);
    let mut sender = UDPSender::new(11201);

    let data: Vec<u8> = vec![0, 1, 2, 3, 4];

    listener.add_listen_address("127.0.0.1", 11201);

    sender.add_send_address("127.0.0.1", 10201);
    sender.send_to_all(&data);

    let rec_data = listener.get_message();

    assert_eq!(rec_data.len(), 5);
    assert_eq!(rec_data, data);
  }

  #[test]
  fn udps_multi_send() {
    let mut listener_1 = UDPListener::new("127.0.0.1", 10301);
    let mut listener_2 = UDPListener::new("127.0.0.1", 10302);
    let mut sender = UDPSender::new(11301);

    let data: Vec<u8> = vec![5, 4, 3, 2, 1, 0];

    listener_1.add_listen_address("127.0.0.1", 11301);
    listener_2.add_listen_address("127.0.0.1", 11301);

    sender.add_send_address("127.0.0.1", 10301);
    sender.add_send_address("127.0.0.1", 10302);
    sender.send_to_all(&data);

    let rec_data_1 = listener_1.get_message();
    let rec_data_2 = listener_2.get_message();

    assert_eq!(rec_data_1.len(), 6);
    assert_eq!(rec_data_1, data);
    assert_eq!(rec_data_2.len(), 6);
    assert_eq!(rec_data_2, data);
  }
}
