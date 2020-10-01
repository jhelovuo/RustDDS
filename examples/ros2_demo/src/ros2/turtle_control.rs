use atosdds::{
  dds::{
    participant::DomainParticipant,
    pubsub::Publisher,
    pubsub::Subscriber,
    topic::Topic,
    typedesc::TypeDesc,
    no_key::{
      datawriter::DataWriter as NoKeyDataWriter, datareader::DataReader as NoKeyDataReader,
    },
  },
  serialization::cdrDeserializer::CDR_deserializer_adapter,
  serialization::cdrSerializer::CDR_serializer_adapter,
};
use byteorder::LittleEndian;

use super::turtle_data::{TurtleCmdVelTopic, Twist, Vector3};

pub struct TurtleControl {
  turtle_cmd_vel_topic: Topic,
  turtle_cmd_vel_publisher: Publisher,
  turtle_cmd_vel_subscriber: Subscriber,
}

impl TurtleControl {
  pub fn new(domain_participant: DomainParticipant) -> TurtleControl {
    let turtle_cmd_vel_topic = domain_participant
      .create_topic(
        &TurtleCmdVelTopic::topic_name(),
        TypeDesc::new(TurtleCmdVelTopic::type_name()),
        &TurtleCmdVelTopic::get_qos(),
      )
      .unwrap();

    let turtle_cmd_vel_publisher = domain_participant
      .create_publisher(&TurtleCmdVelTopic::get_qos())
      .unwrap();
    let turtle_cmd_vel_subscriber = domain_participant
      .create_subscriber(&TurtleCmdVelTopic::get_qos())
      .unwrap();

    TurtleControl {
      turtle_cmd_vel_topic,
      turtle_cmd_vel_publisher,
      turtle_cmd_vel_subscriber,
    }
  }

  pub fn get_cmd_vel_reader(&self) -> NoKeyDataReader<Twist, CDR_deserializer_adapter<Twist>> {
    self
      .turtle_cmd_vel_subscriber
      .create_datareader_no_key(
        None,
        &self.turtle_cmd_vel_topic,
        TurtleCmdVelTopic::get_qos(),
      )
      .unwrap()
  }

  pub fn get_cmd_vel_writer(
    &self,
  ) -> NoKeyDataWriter<Twist, CDR_serializer_adapter<Twist, LittleEndian>> {
    self
      .turtle_cmd_vel_publisher
      .create_datawriter_no_key(
        None,
        &self.turtle_cmd_vel_topic,
        &TurtleCmdVelTopic::get_qos(),
      )
      .unwrap()
  }

  pub fn move_forward() -> Twist {
    Twist {
      linear: Vector3 {
        x: 2.,
        y: 0.,
        z: 0.,
      },
      angular: Vector3 {
        x: 0.,
        y: 0.,
        z: 0.,
      },
    }
  }

  pub fn move_backward() -> Twist {
    Twist {
      linear: Vector3 {
        x: -2.,
        y: 0.,
        z: 0.,
      },
      angular: Vector3 {
        x: 0.,
        y: 0.,
        z: 0.,
      },
    }
  }

  pub fn rotate_left() -> Twist {
    Twist {
      linear: Vector3 {
        x: 0.,
        y: 0.,
        z: 0.,
      },
      angular: Vector3 {
        x: 0.,
        y: 0.,
        z: 2.,
      },
    }
  }

  pub fn rotate_right() -> Twist {
    Twist {
      linear: Vector3 {
        x: 0.,
        y: 0.,
        z: 0.,
      },
      angular: Vector3 {
        x: 0.,
        y: 0.,
        z: -2.,
      },
    }
  }
}
