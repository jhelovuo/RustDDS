use mio::Poll;
use std::{sync::Arc, rc::Rc};

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

use crate::structure::{entity::Entity};

pub struct Discovery {
  poll: Poll,
  domain_participant: Arc<DomainParticipant>,

  discovery_subscriber: Rc<Subscriber>,
  discovery_publisher: Rc<Publisher>,

  dcps_participant_topic: Rc<Topic>,
  dcps_participant_reader: DataReader<SPDPDiscoveredParticipantData>,
  dcps_participant_writer: DataWriter<SPDPDiscoveredParticipantData>,

  dcps_subscription_topic: Rc<Topic>,
  dcps_subscription_reader: DataReader<SPDPDiscoveredParticipantData>,
  dcps_subscription_writer: DataWriter<SPDPDiscoveredParticipantData>,

  dcps_publication_topic: Rc<Topic>,
  dcps_publication_reader: DataReader<SPDPDiscoveredParticipantData>,
  dcps_publication_writer: DataWriter<SPDPDiscoveredParticipantData>,

  dcps_topic: Rc<Topic>,
  dcps_reader: DataReader<SPDPDiscoveredParticipantData>,
  dcps_writer: DataWriter<SPDPDiscoveredParticipantData>,
}

impl Discovery {
  pub fn new(domain_participant: Arc<DomainParticipant>) -> Discovery {
    let poll = mio::Poll::new().expect("Unable to create discovery poll");

    let discovery_subscriber_qos = QosPolicies::qos_none();
    let discovery_subscriber =
      DomainParticipant::create_subscriber(domain_participant.clone(), discovery_subscriber_qos)
        .expect("Unable to create Discovery Subcriber.");

    let discovery_publisher_qos = QosPolicies::qos_none();
    let discovery_publisher =
      DomainParticipant::create_publisher(domain_participant.clone(), discovery_publisher_qos)
        .expect("Unable to create Discovery Publisher.");

    // Participant
    let dcps_participant_qos = Discovery::create_spdp_patricipant_qos();
    let dcps_participant_topic = DomainParticipant::create_topic(
      domain_participant.clone(),
      "DCPSParticipant",
      TypeDesc::new("".to_string()),
      dcps_participant_qos,
    )
    .expect("Unable to create DCPSParticipant topic.");
    let dcps_participant_reader = discovery_subscriber
      .create_datareader(
        domain_participant.get_guid().clone(),
        dcps_participant_topic.get_qos().clone(),
      )
      .expect("Unable to create DataReader for DCPSParticipant");
    let dcps_participant_writer = Publisher::create_datawriter(
      discovery_publisher.clone(),
      dcps_participant_topic.clone(),
      dcps_participant_topic.get_qos().clone(),
    )
    .expect("Unable to create DataWriter for DCPSParticipant.");

    // Subcription
    let dcps_subscription_qos = QosPolicies::qos_none();
    let dcps_subscription_topic = DomainParticipant::create_topic(
      domain_participant.clone(),
      "DCPSSubscription",
      TypeDesc::new("".to_string()),
      dcps_subscription_qos,
    )
    .expect("Unable to create DCPSSubscription topic.");
    let dcps_subscription_reader = discovery_subscriber
      .create_datareader(
        domain_participant.get_guid().clone(),
        dcps_subscription_topic.get_qos().clone(),
      )
      .expect("Unable to create DataReader for DCPSSubscription.");
    let dcps_subscription_writer = Publisher::create_datawriter(
      discovery_publisher.clone(),
      dcps_participant_topic.clone(),
      dcps_participant_topic.get_qos().clone(),
    )
    .expect("Unable to create DataWriter for DCPSSubscription.");

    // Publication
    let dcps_publication_qos = QosPolicies::qos_none();
    let dcps_publication_topic = DomainParticipant::create_topic(
      domain_participant.clone(),
      "DCPSPublication",
      TypeDesc::new("".to_string()),
      dcps_publication_qos,
    )
    .expect("Unable to create DCPSPublication topic.");
    let dcps_publication_reader = discovery_subscriber
      .create_datareader(
        domain_participant.get_guid().clone(),
        dcps_subscription_topic.get_qos().clone(),
      )
      .expect("Unable to create DataReader for DCPSPublication");
    let dcps_publication_writer = Publisher::create_datawriter(
      discovery_publisher.clone(),
      dcps_participant_topic.clone(),
      dcps_participant_topic.get_qos().clone(),
    )
    .expect("Unable to create DataWriter for DCPSPublication.");

    // Topic
    let dcps_topic_qos = QosPolicies::qos_none();
    let dcps_topic = DomainParticipant::create_topic(
      domain_participant.clone(),
      "DCPSTopic",
      TypeDesc::new("".to_string()),
      dcps_topic_qos,
    )
    .expect("Unable to create DCPSTopic topic.");
    let dcps_reader = discovery_subscriber
      .create_datareader(
        domain_participant.get_guid().clone(),
        dcps_subscription_topic.get_qos().clone(),
      )
      .expect("Unable to create DataReader for DCPSTopic");
    let dcps_writer = Publisher::create_datawriter(
      discovery_publisher.clone(),
      dcps_participant_topic.clone(),
      dcps_participant_topic.get_qos().clone(),
    )
    .expect("Unable to create DataWriter for DCPSTopic.");

    Discovery {
      poll,
      domain_participant,
      discovery_subscriber,
      discovery_publisher,
      dcps_participant_topic,
      dcps_participant_reader,
      dcps_participant_writer,
      dcps_subscription_topic,
      dcps_subscription_reader,
      dcps_subscription_writer,
      dcps_publication_topic,
      dcps_publication_reader,
      dcps_publication_writer,
      dcps_topic,
      dcps_reader,
      dcps_writer,
    }
  }

  fn create_spdp_patricipant_qos() -> QosPolicies {
    let mut qos = QosPolicies::qos_none();
    qos.reliability = Some(Reliability::BestEffort);
    qos.history = Some(History::KeepLast { depth: 1 });
    qos
  }

  pub fn discovery_event_loop(_discovery: Discovery) {}
}
