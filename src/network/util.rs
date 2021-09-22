use std::{
  net::{SocketAddr, IpAddr, },
};
use std::io;

#[allow(unused_imports)] use log::{debug, error, info,trace};

use crate::structure::locator::{LocatorList, Locator};

pub fn get_local_multicast_locators(port: u16) -> LocatorList {
  let saddr = SocketAddr::new("239.255.0.1".parse().unwrap(), port);
  vec![Locator::from(saddr)]
}

pub fn get_local_multicast_ip_addrs() -> io::Result<Vec<IpAddr>> {
  let ifs = if_addrs::get_if_addrs()?;
  Ok(ifs.iter()
      .filter( |ifaddr| ( ! ifaddr.is_loopback() ))
      .map( |ifaddr| ifaddr.ip() )
      .collect()
  )
}

pub fn get_local_unicast_socket_address(port: u16) -> LocatorList {
  match if_addrs::get_if_addrs() {
    Ok(ifaces) => {
      ifaces.iter()
        .filter(|ip| ! ip.is_loopback())
        .map(|ip| Locator::from(SocketAddr::new(ip.ip(), port)))
        .collect()
    }
    Err(e) => {
      error!("Cannot get local network interfaces: get_if_addrs() : {:?}",e);
      vec![]
    }
  }
}
