use std::{
  io,
  net::{IpAddr, Ipv4Addr},
};

use if_addrs::Interface;
#[allow(unused_imports)]
use log::{debug, error, info, trace};

use crate::{
  create_error_bad_parameter,
  dds::{CreateError, CreateResult},
};

pub fn get_local_ip_address(only_network: Option<String>) -> CreateResult<IpAddr> {
  let ip_address: IpAddr = if let Some(network_name) = only_network {
    let local_interfaces = if_addrs::get_if_addrs()?;
    match local_interfaces
      .into_iter()
      .find(|interface| interface.name == network_name)
    {
      Some(interface) => interface.addr.ip(),
      None => {
        return create_error_bad_parameter!(
          "Could not find a network interface with the name {network_name}"
        );
      }
    }
  } else {
    // No network specified
    IpAddr::V4(Ipv4Addr::UNSPECIFIED) // Corresponds to 0.0.0.0
  };
  Ok(ip_address)
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
