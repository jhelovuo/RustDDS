use atosdds::{
  dds::qos::{
    QosPolicies, policy::Deadline, policy::DestinationOrder, policy::Durability, policy::History,
    policy::LatencyBudget, policy::Lifespan, policy::Liveliness, policy::LivelinessKind,
    policy::Ownership, policy::Reliability,
  },
  structure::duration::Duration,
};

use serde::{Serialize, Deserialize};
use std::time::Duration as StdDuration;

pub struct TurtleCmdVelTopic {}

impl TurtleCmdVelTopic {
  const QOS: QosPolicies = QosPolicies {
    durability: Some(Durability::Volatile),
    presentation: None,
    deadline: Some(Deadline {
      period: Duration::from_std(StdDuration::from_secs(1)),
    }),
    latency_budget: Some(LatencyBudget {
      duration: Duration::DURATION_ZERO,
    }),
    ownership: Some(Ownership::Shared),
    liveliness: Some(Liveliness {
      kind: LivelinessKind::Automatic,
      lease_duration: Duration::DURATION_INFINITE,
    }),
    time_based_filter: None,
    reliability: Some(Reliability::Reliable {
      max_blocking_time: Duration::DURATION_ZERO,
    }),
    destination_order: Some(DestinationOrder::ByReceptionTimestamp),
    history: Some(History::KeepLast { depth: 10 }),
    resource_limits: None,
    lifespan: Some(Lifespan {
      duration: Duration::DURATION_INFINITE,
    }),
  };

  pub fn topic_name() -> String {
    String::from("rt/turtle1/cmd_vel")
  }

  pub fn type_name() -> String {
    String::from("geometry_msgs::msg::dds_::Twist_")
  }

  pub fn get_qos() -> QosPolicies {
    TurtleCmdVelTopic::QOS
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Twist {
  pub linear: Vector3,
  pub angular: Vector3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector3 {
  pub x: f64,
  pub y: f64,
  pub z: f64,
}

#[cfg(test)]
mod tests {
  use atosdds::{
    dds::traits::serde_adapters::DeserializerAdapter,
    serialization::cdrDeserializer::CDR_deserializer_adapter,
    serialization::cdrSerializer::to_bytes, submessages::RepresentationIdentifier,
  };
  use byteorder::LittleEndian;

  use super::*;
  use std::{fs::File, io::Read};

  #[test]
  fn twist_test() {
    let mut f = File::open("turtle_cmd_vel.bin").unwrap();
    let mut buffer: [u8; 1024] = [0; 1024];
    let len = f.read(&mut buffer).unwrap();

    println!("Buffer: size: {}\n{:?}", len, buffer.to_vec());
    let twist =
      CDR_deserializer_adapter::<Twist>::from_bytes(&buffer, RepresentationIdentifier::CDR_LE)
        .unwrap();
    println!("Twist: \n{:?}", twist);
    let data2 = to_bytes::<Twist, LittleEndian>(&twist).unwrap();
    println!("Data2: \n{:?}", data2);
  }
}
