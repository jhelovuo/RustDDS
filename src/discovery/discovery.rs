use mio::{Ready, Poll, PollOpt, Events};
use mio_extras::timer::Timer;

use std::{
  time::Duration,
  sync::{Arc, RwLock},
};

use crate::dds::{
  participant::DomainParticipant,
  typedesc::TypeDesc,
  qos::{
    QosPolicies, HasQoSPolicy,
    policy::{Reliability, History},
  },
  datareader::DataReader,
};

use crate::discovery::{
  data_types::spdp_participant_data::SPDPDiscoveredParticipantData, discovery_db::DiscoveryDB,
};

use crate::structure::guid::EntityId;

use crate::network::constant::*;

pub struct Discovery {
  poll: Poll,
  domain_participant: DomainParticipant,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
}

impl Discovery {
  const PARTICIPANT_CLEANUP_PERIOD: u64 = 60;

  pub fn new(
    domain_participant: DomainParticipant,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
  ) -> Discovery {
    let poll = mio::Poll::new().expect("Unable to create discovery poll");

    Discovery {
      poll,
      domain_participant,
      discovery_db,
    }
  }

  fn create_spdp_patricipant_qos() -> QosPolicies {
    let mut qos = QosPolicies::qos_none();
    qos.reliability = Some(Reliability::BestEffort);
    qos.history = Some(History::KeepLast { depth: 1 });
    qos
  }

  pub fn discovery_event_loop(discovery: Discovery) {
    let discovery_subscriber_qos = QosPolicies::qos_none();
    let discovery_subscriber = discovery
      .domain_participant
      .create_subscriber(&discovery_subscriber_qos)
      .expect("Unable to create Discovery Subcriber.");

    let discovery_publisher_qos = QosPolicies::qos_none();
    let discovery_publisher = discovery
      .domain_participant
      .create_publisher(&discovery_publisher_qos)
      .expect("Unable to create Discovery Publisher.");

    // Participant
    let dcps_participant_qos = Discovery::create_spdp_patricipant_qos();
    let dcps_participant_topic = discovery
      .domain_participant
      .create_topic(
        "DCPSParticipant",
        TypeDesc::new("".to_string()),
        &dcps_participant_qos,
      )
      .expect("Unable to create DCPSParticipant topic.");

    let dcps_participant_reader = discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData>(
        Some(EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER),
        &dcps_participant_topic,
        dcps_participant_topic.get_qos(),
      )
      .expect("Unable to create DataReader for DCPSParticipant");
    // register participant reader
    discovery
      .poll
      .register(
        &dcps_participant_reader,
        DISCOVERY_PARTICIPANT_DATA_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register participant reader to poll.");

    // create lease duration check timer
    let mut participant_cleanup_timer: Timer<()> = Timer::default();
    participant_cleanup_timer.set_timeout(
      Duration::from_secs(Discovery::PARTICIPANT_CLEANUP_PERIOD),
      (),
    );
    discovery
      .poll
      .register(
        &participant_cleanup_timer,
        DISCOVERY_PARTICIPANT_CLEANUP_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to create participant cleanup timer");

    let _dcps_participant_writer = discovery_publisher
      .create_datawriter::<SPDPDiscoveredParticipantData>(
        Some(EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER),
        &dcps_participant_topic,
        dcps_participant_topic.get_qos(),
      )
      .expect("Unable to create DataWriter for DCPSParticipant.");

    // Subcription
    let dcps_subscription_qos = QosPolicies::qos_none();
    let dcps_subscription_topic = discovery
      .domain_participant
      .create_topic(
        "DCPSSubscription",
        TypeDesc::new("".to_string()),
        &dcps_subscription_qos,
      )
      .expect("Unable to create DCPSSubscription topic.");

    let _dcps_subscription_reader = discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER),
        &dcps_subscription_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataReader for DCPSSubscription.");

    let _dcps_subscription_writer = discovery_publisher
      .create_datawriter::<SPDPDiscoveredParticipantData>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER),
        &dcps_subscription_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataWriter for DCPSSubscription.");

    // Publication
    let dcps_publication_qos = QosPolicies::qos_none();
    let dcps_publication_topic = discovery
      .domain_participant
      .create_topic(
        "DCPSPublication",
        TypeDesc::new("".to_string()),
        &dcps_publication_qos,
      )
      .expect("Unable to create DCPSPublication topic.");

    let _dcps_publication_reader = discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER),
        &dcps_publication_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataReader for DCPSPublication");
    let _dcps_publication_writer = discovery_publisher
      .create_datawriter::<SPDPDiscoveredParticipantData>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER),
        &dcps_publication_topic,
        dcps_publication_topic.get_qos(),
      )
      .expect("Unable to create DataWriter for DCPSPublication.");

    // Topic
    let dcps_topic_qos = QosPolicies::qos_none();
    let dcps_topic = discovery
      .domain_participant
      .create_topic("DCPSTopic", TypeDesc::new("".to_string()), &dcps_topic_qos)
      .expect("Unable to create DCPSTopic topic.");
    let _dcps_reader = discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER),
        &dcps_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataReader for DCPSTopic");
    let _dcps_writer = discovery_publisher
      .create_datawriter::<SPDPDiscoveredParticipantData>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER),
        &dcps_topic,
        dcps_topic.get_qos(),
      )
      .expect("Unable to create DataWriter for DCPSTopic.");

    loop {
      let mut events = Events::with_capacity(1024);
      discovery
        .poll
        .poll(&mut events, None)
        .expect("Failed in waiting of poll.");
      for event in events.into_iter() {
        if event.token() == STOP_POLL_TOKEN {
          return;
        } else if event.token() == DISCOVERY_PARTICIPANT_DATA_TOKEN {
          discovery.handle_participant_reader(&dcps_participant_reader);
        } else if event.token() == DISCOVERY_PARTICIPANT_CLEANUP_TOKEN {
          discovery.participant_cleanup();
          // setting next cleanup timeout
          participant_cleanup_timer.set_timeout(
            Duration::from_secs(Discovery::PARTICIPANT_CLEANUP_PERIOD),
            (),
          );
        }
      }
    }
  }

  pub fn handle_participant_reader(&self, reader: &DataReader<SPDPDiscoveredParticipantData>) {
    let participant_datas = match reader.read_next() {
      Ok(d) => d,
      _ => return (),
    };

    for pd in participant_datas.iter() {
      let dbres = self.discovery_db.write();
      match dbres {
        Ok(mut db) => {
          (*db).update_participant(pd);
        }
        _ => continue,
      }
      // debug for when all parts are available
      println!("Participant: {:?}", pd);
    }
  }

  pub fn participant_cleanup(&self) {
    let dbres = self.discovery_db.write();
    match dbres {
      Ok(mut db) => {
        (*db).participant_cleanup();
      }
      _ => return (),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::network::udp_sender::UDPSender;
  use std::{time::Duration, net::SocketAddr};

  #[test]
  fn discovery_participant_test() {
    let _participant = DomainParticipant::new(0, 0);

    // sending participant data to discovery
    let udp_sender = UDPSender::new_with_random_port();
    let addresses = vec![SocketAddr::new("127.0.0.1".parse().unwrap(), 7412)];

    const data: [u8; 204] = [
      // Offset 0x00000000 to 0x00000203
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x09, 0x01, 0x08, 0x00, 0x0e, 0x15, 0xf3, 0x5e, 0x00, 0x28,
      0x74, 0xd2, 0x15, 0x05, 0xa8, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x01, 0x00, 0xc7, 0x00,
      0x01, 0x00, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
      0x15, 0x00, 0x04, 0x00, 0x02, 0x03, 0x00, 0x00, 0x16, 0x00, 0x04, 0x00, 0x01, 0x0f, 0x00,
      0x00, 0x50, 0x00, 0x10, 0x00, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x01, 0xc1, 0x32, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xf4,
      0x1c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x0a, 0x50, 0x8e, 0x68, 0x31, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xf5, 0x1c, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x50,
      0x8e, 0x68, 0x02, 0x00, 0x08, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x58,
      0x00, 0x04, 0x00, 0x3f, 0x0c, 0x3f, 0x0c, 0x62, 0x00, 0x18, 0x00, 0x14, 0x00, 0x00, 0x00,
      0x66, 0x61, 0x73, 0x74, 0x72, 0x74, 0x70, 0x73, 0x50, 0x61, 0x72, 0x74, 0x69, 0x63, 0x69,
      0x70, 0x61, 0x6e, 0x74, 0x00, 0x01, 0x00, 0x00, 0x00,
    ];

    udp_sender.send_to_all(&data, &addresses);

    // during the 5 secs data should be visible
    std::thread::sleep(Duration::from_secs(5));
  }
}
