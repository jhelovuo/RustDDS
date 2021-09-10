use std::{
  net::{SocketAddr, IpAddr},
  io::Error,
};

use crate::structure::locator::{LocatorList, Locator};

pub fn get_local_multicast_locators(port: u16) -> LocatorList {
  let saddr = SocketAddr::new("239.255.0.1".parse().unwrap(), port);
  vec![Locator::from(saddr)]
}

pub fn get_local_unicast_socket_address(port: u16) -> LocatorList {
  let local_ips: Result<Vec<IpAddr>, Error> = if_addrs::get_if_addrs().map(|p| {
    p.iter()
      .filter(|ip| !ip.is_loopback())
      .map(|ip| ip.ip())
      .collect()
  });

  match local_ips {
    Ok(ips) => {
      let loc = ips
        .into_iter()
        .map(|p| SocketAddr::new(p, port))
        .map(|p| Locator::from(p))
        .next();
      match loc {
        Some(l) => vec![l],
        None => vec![],
      }
    }
    _ => vec![],
  }
}
