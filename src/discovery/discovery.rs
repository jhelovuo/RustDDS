use mio::Poll;

use crate::dds::{
  participant::DomainParticipant,
  topic::Topic,
  typedesc::TypeDesc,
  qos::{
    QosPolicies, HasQoSPolicy,
    policy::{Reliability, History},
  },
  datawriter::DataWriter,
  datareader::DataReader,
  pubsub::{Subscriber, Publisher},
};

use crate::discovery::data_types::spdp_participant_data::SPDPDiscoveredParticipantData;

use crate::structure::entity::Entity;

pub struct Discovery<'a> {
  poll: Poll,
  domain_participant: DomainParticipant,

  discovery_subscriber: Subscriber,
  discovery_publisher: Publisher,

  dcps_participant_topic: Topic,
  dcps_participant_reader: DataReader<'a, SPDPDiscoveredParticipantData>,
  dcps_participant_writer: DataWriter<'a, SPDPDiscoveredParticipantData>,

  dcps_subscription_topic: Topic,
  dcps_subscription_reader: DataReader<'a, SPDPDiscoveredParticipantData>,
  dcps_subscription_writer: DataWriter<'a, SPDPDiscoveredParticipantData>,

  dcps_publication_topic: Topic,
  dcps_publication_reader: DataReader<'a, SPDPDiscoveredParticipantData>,
  dcps_publication_writer: DataWriter<'a, SPDPDiscoveredParticipantData>,

  dcps_topic: Topic,
  dcps_reader: DataReader<'a, SPDPDiscoveredParticipantData>,
  dcps_writer: DataWriter<'a, SPDPDiscoveredParticipantData>,
}

impl<'a> Discovery<'a> {
  pub fn new(domain_participant: &DomainParticipant) {
    let _poll = mio::Poll::new().expect("Unable to create discovery poll");

    let discovery_subscriber_qos = QosPolicies::qos_none();
    let discovery_subscriber = domain_participant
      .create_subscriber(&discovery_subscriber_qos)
      .expect("Unable to create Discovery Subcriber.");

    let discovery_publisher_qos = QosPolicies::qos_none();
    let discovery_publisher = domain_participant
      .create_publisher(&discovery_publisher_qos)
      .expect("Unable to create Discovery Publisher.");

    // Participant
    let dcps_participant_qos = Discovery::create_spdp_patricipant_qos();
    let dcps_participant_topic = domain_participant
      .create_topic(
        "DCPSParticipant",
        TypeDesc::new("".to_string()),
        &dcps_participant_qos,
      )
      .expect("Unable to create DCPSParticipant topic.");

    let _dcps_participant_reader = discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData>(
        &dcps_participant_topic,
        dcps_participant_topic.get_qos(),
      )
      .expect("Unable to create DataReader for DCPSParticipant");

    let _dcps_participant_writer = discovery_publisher
      .create_datawriter::<SPDPDiscoveredParticipantData>(
        &dcps_participant_topic,
        dcps_participant_topic.get_qos(),
      )
      .expect("Unable to create DataWriter for DCPSParticipant.");

    // Subcription
    let dcps_subscription_qos = QosPolicies::qos_none();
    let dcps_subscription_topic = domain_participant
      .create_topic(
        "DCPSSubscription",
        TypeDesc::new("".to_string()),
        &dcps_subscription_qos,
      )
      .expect("Unable to create DCPSSubscription topic.");

    let _dcps_subscription_reader = discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData>(
        &dcps_subscription_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataReader for DCPSSubscription.");

    let _dcps_subscription_writer = discovery_publisher
      .create_datawriter::<SPDPDiscoveredParticipantData>(
        &dcps_subscription_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataWriter for DCPSSubscription.");

    // Publication
    let dcps_publication_qos = QosPolicies::qos_none();
    let dcps_publication_topic = domain_participant
      .create_topic(
        "DCPSPublication",
        TypeDesc::new("".to_string()),
        &dcps_publication_qos,
      )
      .expect("Unable to create DCPSPublication topic.");

    let _dcps_publication_reader = discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData>(
        &dcps_publication_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataReader for DCPSPublication");
    let _dcps_publication_writer = discovery_publisher
      .create_datawriter::<SPDPDiscoveredParticipantData>(
        &dcps_publication_topic,
        dcps_publication_topic.get_qos(),
      )
      .expect("Unable to create DataWriter for DCPSPublication.");

    // Topic
    let dcps_topic_qos = QosPolicies::qos_none();
    let dcps_topic = domain_participant
      .create_topic("DCPSTopic", TypeDesc::new("".to_string()), &dcps_topic_qos)
      .expect("Unable to create DCPSTopic topic.");
    let _dcps_reader = discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData>(
        &dcps_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataReader for DCPSTopic");
    let _dcps_writer = discovery_publisher
      .create_datawriter::<SPDPDiscoveredParticipantData>(&dcps_topic, dcps_topic.get_qos())
      .expect("Unable to create DataWriter for DCPSTopic.");

    // Discovery {
    //   poll,
    //   domain_participant: domain_participant.clone(),
    //   discovery_subscriber,
    //   discovery_publisher,
    //   dcps_participant_topic,
    //   dcps_participant_reader,
    //   dcps_participant_writer,
    //   dcps_subscription_topic,
    //   dcps_subscription_reader,
    //   dcps_subscription_writer,
    //   dcps_publication_topic,
    //   dcps_publication_reader,
    //   dcps_publication_writer,
    //   dcps_topic,
    //   dcps_reader,
    //   dcps_writer,
    // }
  }

  fn create_spdp_patricipant_qos() -> QosPolicies {
    let mut qos = QosPolicies::qos_none();
    qos.reliability = Some(Reliability::BestEffort);
    qos.history = Some(History::KeepLast { depth: 1 });
    qos
  }

  pub fn discovery_event_loop(_discovery: Discovery) {}
}
