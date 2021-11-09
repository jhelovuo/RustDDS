use rustdds::dds::{
  data_types::{DDSDuration, TopicKind},
  qos::{
    policy::{
      Deadline, DestinationOrder, Durability, History, LatencyBudget, Lifespan, Liveliness,
      Ownership, Reliability,
    },
    QosPolicies, QosPolicyBuilder,
  },
};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;

lazy_static! {
  static ref QOS: QosPolicies = {
    QosPolicyBuilder::new()
      .durability(Durability::Volatile)
      .deadline(Deadline(DDSDuration::DURATION_INFINITE))
      .latency_budget(LatencyBudget {
        duration: DDSDuration::DURATION_ZERO,
      })
      .ownership(Ownership::Shared)
      .liveliness(Liveliness::Automatic {
        lease_duration: DDSDuration::DURATION_INFINITE,
      })
      .reliability(Reliability::Reliable {
        max_blocking_time: DDSDuration::DURATION_ZERO,
      })
      .destination_order(DestinationOrder::ByReceptionTimestamp)
      .history(History::KeepLast { depth: 10 })
      .lifespan(Lifespan {
        duration: DDSDuration::DURATION_INFINITE,
      })
      .build()
  };
}

pub struct TurtleCmdVelTopic {}

impl TurtleCmdVelTopic {
  pub fn topic_name() -> String {
    String::from("/turtle1/cmd_vel")
  }

  pub fn topic_kind() -> TopicKind {
    TopicKind::NoKey
  }

  pub fn type_name() -> String {
    String::from("geometry_msgs::msg::dds_::Twist_")
  }

  pub fn qos() -> QosPolicies {
    QOS.clone()
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
  use std::{fs::File, io::Read};

  use rustdds::{
    dds::traits::serde_adapters::DeserializerAdapter,
    serialization::{cdr_deserializer::CDRDeserializerAdapter, cdr_serializer::to_bytes},
    submessages::RepresentationIdentifier,
  };
  use byteorder::LittleEndian;

  use super::*;

  #[test]
  fn twist_test() {
    let mut f = File::open("turtle_cmd_vel.bin").unwrap();
    let mut buffer: [u8; 1024] = [0; 1024];
    let len = f.read(&mut buffer).unwrap();

    println!("Buffer: size: {}\n{:?}", len, buffer.to_vec());
    let twist =
      CDRDeserializerAdapter::<Twist>::from_bytes(&buffer, RepresentationIdentifier::CDR_LE)
        .unwrap();
    println!("Twist: \n{:?}", twist);
    let data2 = to_bytes::<Twist, LittleEndian>(&twist).unwrap();
    println!("Data2: \n{:?}", data2);
  }
}
