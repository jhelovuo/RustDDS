use std::{
  io,
  net::{IpAddr, Ipv4Addr, SocketAddrV4},
};

use if_addrs::Interface;
#[allow(unused_imports)]
use log::{debug, error, info, trace};

#[derive(Clone)]
pub struct UnicastInfo {
  pub host_ip: Ipv4Addr,
  pub host_discovery_port: u16,
  pub remote_discovery_addrs: Vec<SocketAddrV4>,
}

#[derive(Clone)]
pub enum NetworkInfo {
  Multicast(Ipv4Addr),  // Supports multicast. Contains the host IP address to use.
  Unicast(UnicastInfo), // Unicast only
}

impl NetworkInfo {
  pub fn new_multicast(host_ip_addr: Ipv4Addr) -> Self {
    Self::Multicast(host_ip_addr)
  }

  pub fn unspecified_multicast() -> Self {
    Self::Multicast(Ipv4Addr::UNSPECIFIED)
  }

  pub fn new_unicast_only(
    host_ip: Ipv4Addr,
    host_discovery_port: u16,
    remote_discovery_addrs: Vec<SocketAddrV4>,
  ) -> Self {
    Self::Unicast(UnicastInfo {
      host_ip,
      host_discovery_port,
      remote_discovery_addrs,
    })
  }

  pub fn host_ip(&self) -> Ipv4Addr {
    match self {
      NetworkInfo::Multicast(ip_addr) => *ip_addr,
      NetworkInfo::Unicast(unicast_info) => unicast_info.host_ip,
    }
  }
}

pub fn get_local_nonloopback_ip_addrs() -> io::Result<Vec<IpAddr>> {
  let ifs = if_addrs::get_if_addrs()?;
  Ok(
    ifs
      .iter()
      .map(Interface::ip)
      .filter(|ip| !ip.is_loopback())
      .collect(),
  )
}
