#![allow(unused_imports)]
#![allow(dead_code)]

use std::{collections::HashSet, io};

use log4rs::{
  append::console::ConsoleAppender,
  config::{Appender, Root},
  Config,
};
use log::{debug, error, info, warn, LevelFilter};
use mio_06::{Events, Poll, PollOpt, Ready, Token}; // polling
use mio_extras::channel; // pollable channel
use clap::{Arg, ArgMatches, Command};
use rustdds::{DomainParticipant, DomainParticipantBuilder, GuidPrefix, RTPSEntity, GUID};
#[cfg(feature = "rtps_proxy")]
use rustdds::rtps_proxy::{DiscoverySample, ProxyDataEndpoint, RTPSMessage};
use socket2::Domain;

const STOP_PROGRAM: Token = Token(0);
const DP1_RTPS_DATA: Token = Token(1);
const DP1_DISCOVERY_DATA: Token = Token(2);
const DP2_RTPS_DATA: Token = Token(3);
const DP2_DISCOVERY_DATA: Token = Token(4);

#[cfg(feature = "security")]
use std::path::Path;

#[cfg(feature = "security")]
use rustdds::DomainParticipantSecurityConfigFiles;

#[cfg(feature = "rtps_proxy")]
pub fn relay_discovery_sample(
  sample: DiscoverySample,
  target_endpoint: &ProxyDataEndpoint<DiscoverySample>,
) {
  if let Err(e) = target_endpoint.try_send(sample) {
    println!("Error relaying DiscoverySample: {e}");
  }
}

#[cfg(feature = "rtps_proxy")]
fn relay_rtps_message(msg: RTPSMessage, target_endpoint: &ProxyDataEndpoint<RTPSMessage>) {
  if let Err(e) = target_endpoint.try_send(msg) {
    println!("Error relaying RTSP message: {e}");
  }
}

#[cfg(not(feature = "rtps_proxy"))]
fn main() {
  println!("Feature rtps_proxy is not enabled!");
}

#[cfg(feature = "rtps_proxy")]
fn main() {
  configure_logging();
  let matches = get_matches();
  let dp1_builder = DomainParticipantBuilder::new(0);
  let dp2_builder = DomainParticipantBuilder::new(0);

  let multicast_ip_addr1 = "172.30.80.31";
  let dp1_builder = dp1_builder.use_single_multicast_network(multicast_ip_addr1);

  let local_discovery_addr2 = "172.30.80.31";
  let local_discovery_port2 = 7510;
  let remote_discovery_addrs2 = vec!["172.30.80.31:7610".to_string()];
  let dp2_builder = dp2_builder.use_single_unicast_network(
    local_discovery_addr2,
    local_discovery_port2,
    remote_discovery_addrs2,
  );

  #[cfg(feature = "security")]
  let (dp1_builder, dp2_builder) = if let Some(sec_dir_path) = matches.get_one::<String>("security")
  {
    let dp1_builder = dp1_builder.builtin_security(
      DomainParticipantSecurityConfigFiles::with_ros_default_names(
        Path::new(sec_dir_path),
        "no_pwd".to_string(),
      ),
    );
    let dp2_builder = dp2_builder.builtin_security(
      DomainParticipantSecurityConfigFiles::with_ros_default_names(
        Path::new(sec_dir_path),
        "no_pwd".to_string(),
      ),
    );
    (dp1_builder, dp2_builder)
  } else {
    (dp1_builder, dp2_builder)
  };

  let dp1 = dp1_builder
    .build()
    .unwrap_or_else(|e| panic!("DomainParticipant construction failed: {e:?}"));

  let dp2 = dp2_builder
    .build()
    .unwrap_or_else(|e| panic!("DomainParticipant construction failed: {e:?}"));

  let dp1_proxy_endpoints_ref = dp1.proxy_app_endpoints();
  let dp1_proxy_endpoints = dp1_proxy_endpoints_ref.lock().unwrap();

  let dp2_proxy_endpoints_ref = dp2.proxy_app_endpoints();
  let dp2_proxy_endpoints = dp2_proxy_endpoints_ref.lock().unwrap();

  info!("DP1: {:?}", dp1.guid_prefix());
  info!("DP2: {:?}", dp2.guid_prefix());

  // Set Ctrl-C handler
  let (stop_sender, stop_receiver) = channel::channel();
  ctrlc::set_handler(move || {
    stop_sender.send(()).unwrap_or(());
    // ignore errors, as we are quitting anyway
  })
  .expect("Error setting Ctrl-C handler");
  println!("Press Ctrl-C to quit.");

  let poll = Poll::new().unwrap();
  let mut events = Events::with_capacity(4);

  poll
    .register(
      &stop_receiver,
      STOP_PROGRAM,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  poll
    .register(
      &dp1_proxy_endpoints.rtps,
      DP1_RTPS_DATA,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  poll
    .register(
      &dp1_proxy_endpoints.discovery,
      DP1_DISCOVERY_DATA,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  poll
    .register(
      &dp2_proxy_endpoints.rtps,
      DP2_RTPS_DATA,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  poll
    .register(
      &dp2_proxy_endpoints.discovery,
      DP2_DISCOVERY_DATA,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  loop {
    poll.poll(&mut events, None).unwrap();
    for event in &events {
      match event.token() {
        STOP_PROGRAM => {
          if stop_receiver.try_recv().is_ok() {
            println!("Done.");
            return;
          }
        }
        DP1_RTPS_DATA => {
          while let Some(msg) = dp1_proxy_endpoints.rtps.try_recv_data() {
            relay_rtps_message(msg, &dp2_proxy_endpoints.rtps);
          }
        }
        DP1_DISCOVERY_DATA => {
          while let Some(sample) = dp1_proxy_endpoints.discovery.try_recv_data() {
            relay_discovery_sample(sample, &dp2_proxy_endpoints.discovery);
          }
        }
        DP2_RTPS_DATA => {
          while let Some(msg) = dp2_proxy_endpoints.rtps.try_recv_data() {
            relay_rtps_message(msg, &dp1_proxy_endpoints.rtps);
          }
        }
        DP2_DISCOVERY_DATA => {
          while let Some(sample) = dp2_proxy_endpoints.discovery.try_recv_data() {
            relay_discovery_sample(sample, &dp1_proxy_endpoints.discovery);
          }
        }
        other_token => {
          println!("Polled event is {other_token:?}. WTF?");
        }
      }
      // println!()
    }
  } // loop
}

#[allow(clippy::too_many_lines)]
fn get_matches() -> ArgMatches {
  Command::new("RustDDS-interop")
    .version("0.2.2")
    .author("Juhana Helovuo <juhe@iki.fi>")
    .about("Command-line \"shapes\" interoperability test.")
    .arg(
      Arg::new("security")
        .help(
          "Path to directory containing security configuration files. Setting this enables \
           security.",
        )
        .long("security")
        .value_name("security"),
    )
    .arg(
      Arg::new("pkcs11-library")
        .help("Path to a library implementing PKCS#11 client.")
        .long("pkcs11-library")
        .value_name("pkcs11-library")
        .requires("pkcs11-token"),
    )
    .arg(
      Arg::new("pkcs11-token")
        .help("Token label for PKCS#11")
        .long("pkcs11-token")
        .value_name("pkcs11-token")
        .requires("security")
        .requires("pkcs11-library"),
    )
    .arg(
      Arg::new("pkcs11-pin")
        .help("PIN to access PKCS#11 token")
        .long("pkcs11-pin")
        .value_name("pkcs11-pin")
        .requires("pkcs11-token"),
    )
    .get_matches()
}

fn configure_logging() {
  // initialize logging, preferably from config file
  log4rs::init_file(
    "logging-config.yaml",
    log4rs::config::Deserializers::default(),
  )
  .unwrap_or_else(|e| {
    match e.downcast_ref::<io::Error>() {
      // Config file did not work. If it is a simple "No such file or directory", then
      // substitute some default config.
      Some(os_err) if os_err.kind() == io::ErrorKind::NotFound => {
        println!("No config file found in current working directory.");
        let stdout = ConsoleAppender::builder().build();
        let conf = Config::builder()
          .appender(Appender::builder().build("stdout", Box::new(stdout)))
          .build(Root::builder().appender("stdout").build(LevelFilter::Error))
          .unwrap();
        log4rs::init_config(conf).unwrap();
      }
      // Give up.
      other_error => panic!("Config problem: {other_error:?}"),
    }
  });
}
