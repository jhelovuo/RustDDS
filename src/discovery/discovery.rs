use mio::{Ready, Poll, PollOpt, Events};
use mio_extras::timer::Timer;
use mio_extras::channel as mio_channel;

use std::{
  time::Duration as StdDuration,
  sync::{Arc, RwLock},
};

use crate::dds::{
  participant::DomainParticipant,
  typedesc::TypeDesc,
  qos::{
    QosPolicies, HasQoSPolicy,
    policy::{Reliability, History},
  },
  datareader::{Take, DataReader},
  datawriter::{DataWriter},
  readcondition::ReadCondition,
};

use crate::discovery::{
  data_types::spdp_participant_data::SPDPDiscoveredParticipantData, discovery_db::DiscoveryDB,
};

use crate::structure::{guid::EntityId, entity::Entity};

use crate::network::constant::*;
use super::data_types::topic_data::DiscoveredReaderData;

pub struct Discovery {
  poll: Poll,
  domain_participant: DomainParticipant,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  writers_proxy_updated_sender: mio_channel::Sender<()>,
  readers_proxy_updated_sender: mio_channel::Sender<()>,
}

unsafe impl Sync for Discovery {}
unsafe impl Send for Discovery {}

impl Discovery {
  const PARTICIPANT_CLEANUP_PERIOD: u64 = 60;
  const SEND_PARTICIPANT_INFO_PERIOD: u64 = 1;

  pub fn new(
    domain_participant: DomainParticipant,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    writers_proxy_updated_sender: mio_channel::Sender<()>,
    readers_proxy_updated_sender: mio_channel::Sender<()>,
  ) -> Discovery {
    let poll = mio::Poll::new().expect("Unable to create discovery poll");

    Discovery {
      poll,
      domain_participant,
      discovery_db,
      writers_proxy_updated_sender,
      readers_proxy_updated_sender,
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

    let mut dcps_participant_reader = discovery_subscriber
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
      StdDuration::from_secs(Discovery::PARTICIPANT_CLEANUP_PERIOD),
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

    let mut dcps_participant_writer = discovery_publisher
      .create_datawriter::<SPDPDiscoveredParticipantData>(
        Some(EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER),
        &dcps_participant_topic,
        dcps_participant_topic.get_qos(),
      )
      .expect("Unable to create DataWriter for DCPSParticipant.");

    // creating timer for sending out own participant data
    let mut participant_send_info_timer: Timer<()> = Timer::default();
    participant_send_info_timer.set_timeout(
      StdDuration::from_secs(Discovery::SEND_PARTICIPANT_INFO_PERIOD),
      (),
    );
    discovery
      .poll
      .register(
        &participant_send_info_timer,
        DISCOVERY_SEND_PARTICIPANT_INFO_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to register participant info sneder");

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

    let mut dcps_subscription_reader = discovery_subscriber
      .create_datareader::<DiscoveredReaderData>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER),
        &dcps_subscription_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataReader for DCPSSubscription.");
    discovery
      .poll
      .register(
        &dcps_subscription_reader,
        DISCOVERY_SUBSCRIPTION_DATA_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to register subscription reader.");

    let _dcps_subscription_writer = discovery_publisher
      .create_datawriter::<DiscoveredReaderData>(
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
          let data = discovery.handle_participant_reader(&mut dcps_participant_reader);
          match data {
            Some(dat) => {
              discovery.update_spdp_participant_writer(dat, &dcps_participant_writer);
            }
            None => (),
          }
        } else if event.token() == DISCOVERY_PARTICIPANT_CLEANUP_TOKEN {
          discovery.participant_cleanup();
          // setting next cleanup timeout
          participant_cleanup_timer.set_timeout(
            StdDuration::from_secs(Discovery::PARTICIPANT_CLEANUP_PERIOD),
            (),
          );
        } else if event.token() == DISCOVERY_SEND_PARTICIPANT_INFO_TOKEN {
          // setting 3 times the duration so lease doesn't break if we fail once for some reason
          let lease_duration = StdDuration::from_secs(Discovery::SEND_PARTICIPANT_INFO_PERIOD * 3);
          let data = SPDPDiscoveredParticipantData::from_participant(
            &discovery.domain_participant,
            lease_duration,
          );
          dcps_participant_writer.write(data, None).unwrap_or(());
          // reschedule timer
          participant_send_info_timer.set_timeout(
            StdDuration::from_secs(Discovery::SEND_PARTICIPANT_INFO_PERIOD),
            (),
          );
        } else if event.token() == DISCOVERY_SUBSCRIPTION_DATA_TOKEN {
          discovery.handle_subscription_reader(&mut dcps_subscription_reader);
        }
      }
    }
  }

  pub fn handle_participant_reader(
    &self,
    reader: &mut DataReader<SPDPDiscoveredParticipantData>,
  ) -> Option<SPDPDiscoveredParticipantData> {
    let participant_data = match reader.read_next_sample(Take::Yes) {
      Ok(d) => match d {
        Some(d) => match &d.value {
          Ok(aaaaa) => (aaaaa).clone(),
          _ => return None,
        },
        None => return None,
      },
      _ => return None,
    };

    let dbres = self.discovery_db.write();
    match dbres {
      Ok(mut db) => {
        let updated = (*db).update_participant(&participant_data);
        if updated {
          println!("Update participant data.");
          return Some(participant_data);
        }
      }
      _ => return None,
    }
    // debug for when all parts are available
    println!("Participant: {:?}", participant_data);
    None
  }

  pub fn handle_subscription_reader(&self, reader: &mut DataReader<DiscoveredReaderData>) {
    let reader_data_vec = match reader.take(100, ReadCondition::not_read()) {
      Ok(d) => Some(
        d.into_iter()
          .map(|p| p.value)
          .filter(|p| p.is_ok())
          .map(|p| p.unwrap())
          .collect(),
      ),
      _ => None,
    };

    let reader_data_vec: Vec<DiscoveredReaderData> = match reader_data_vec {
      Some(d) => d,
      None => return,
    };

    let _res = self.discovery_db.write().map(|mut p| {
      for data in reader_data_vec.iter() {
        let updated = (*p).update_subscription(data);
        if updated {
          let _send_result = self.writers_proxy_updated_sender.send(());
          println!("Updated subscription {}", updated);
        }
      }
    });
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

  pub fn update_spdp_participant_writer(
    &self,
    data: SPDPDiscoveredParticipantData,
    writer: &DataWriter<SPDPDiscoveredParticipantData>,
  ) -> bool {
    // TODO: remove unwrap and handle error
    let new_proxy = data.as_reader_proxy().unwrap();
    let dbres = self.discovery_db.write();
    match dbres {
      Ok(mut db) => (*db).update_writers_reader_proxy(writer.get_guid(), new_proxy),
      _ => return false,
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
