use atosdds::{
  dds::no_key::{
    datawriter::DataWriter as NoKeyDataWriter, datareader::DataReader as NoKeyDataReader,
  },
  dds::participant::DomainParticipant,
  dds::pubsub::Publisher,
  dds::pubsub::Subscriber,
  dds::topic::Topic,
  dds::typedesc::TypeDesc,
  serialization::cdrDeserializer::CDR_deserializer_adapter,
  serialization::cdrSerializer::CDR_serializer_adapter,
};
use byteorder::LittleEndian;

use crate::{
  ros_data::{
    Log, ParameterEvents, ParameterEventsTopic, ROSDiscoveryTopic, ROSParticipantInfo, RosOutTopic,
  },
};

pub struct NodeControl {
  node_topic: Topic,
  node_subscriber: Subscriber,
  node_publisher: Publisher,
  parameter_events_topic: Topic,
  parameter_events_publisher: Publisher,
  rosout_topic: Topic,
  rosout_publisher: Publisher,
}

impl NodeControl {
  pub fn new(domain_participant: DomainParticipant) -> NodeControl {
    let ros_discovery_topic = domain_participant
      .create_topic(
        &ROSDiscoveryTopic::topic_name(),
        TypeDesc::new(ROSDiscoveryTopic::type_name()),
        &ROSDiscoveryTopic::get_qos(),
      )
      .unwrap();

    let node_subscriber = domain_participant
      .create_subscriber(&ROSDiscoveryTopic::get_qos())
      .unwrap();
    let node_publisher = domain_participant
      .create_publisher(&ROSDiscoveryTopic::get_qos())
      .unwrap();

    let parameter_events_topic = domain_participant
      .create_topic(
        &ParameterEventsTopic::topic_name(),
        TypeDesc::new(ParameterEventsTopic::type_name()),
        &ParameterEventsTopic::get_qos(),
      )
      .unwrap();

    let parameter_events_publisher = domain_participant
      .create_publisher(&ParameterEventsTopic::get_qos())
      .unwrap();

    let rosout_topic = domain_participant
      .create_topic(
        &RosOutTopic::topic_name(),
        TypeDesc::new(RosOutTopic::type_name()),
        &RosOutTopic::get_qos(),
      )
      .unwrap();

    let rosout_publisher = domain_participant
      .create_publisher(&RosOutTopic::get_qos())
      .unwrap();

    NodeControl {
      node_topic: ros_discovery_topic,
      node_subscriber: node_subscriber,
      node_publisher: node_publisher,
      parameter_events_topic,
      parameter_events_publisher,
      rosout_topic,
      rosout_publisher,
    }
  }

  pub fn get_node_reader(
    &self,
  ) -> NoKeyDataReader<ROSParticipantInfo, CDR_deserializer_adapter<ROSParticipantInfo>> {
    self
      .node_subscriber
      .create_datareader_no_key::<ROSParticipantInfo, CDR_deserializer_adapter<ROSParticipantInfo>>(
        None,
        &self.node_topic,
        ROSDiscoveryTopic::get_qos(),
      )
      .unwrap()
  }

  pub fn get_node_writer(
    &self,
  ) -> NoKeyDataWriter<ROSParticipantInfo, CDR_serializer_adapter<ROSParticipantInfo, LittleEndian>>
  {
    self.node_publisher
    .create_datawriter_no_key::<ROSParticipantInfo, CDR_serializer_adapter<ROSParticipantInfo, LittleEndian>>(
      None,
      &self.node_topic,
      &ROSDiscoveryTopic::get_qos(),
    )
    .unwrap()
  }

  pub fn get_parameter_events_writer(
    &self,
  ) -> NoKeyDataWriter<ParameterEvents, CDR_serializer_adapter<ParameterEvents, LittleEndian>> {
    self.parameter_events_publisher.create_datawriter_no_key::<ParameterEvents, CDR_serializer_adapter<ParameterEvents, LittleEndian>>(None, &self.parameter_events_topic, &ParameterEventsTopic::get_qos()).unwrap()
  }

  pub fn get_rosout_writer(
    &self,
  ) -> NoKeyDataWriter<Log, CDR_serializer_adapter<Log, LittleEndian>> {
    self
      .rosout_publisher
      .create_datawriter_no_key::<Log, CDR_serializer_adapter<Log, LittleEndian>>(
        None,
        &self.rosout_topic,
        &RosOutTopic::get_qos(),
      )
      .unwrap()
  }
}
