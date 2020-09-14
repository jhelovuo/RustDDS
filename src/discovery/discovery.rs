use mio::{Ready, Poll, PollOpt, Events};
use mio_extras::timer::Timer;
use mio_extras::channel as mio_channel;

use std::{
  time::Duration as StdDuration,
  sync::{Arc, RwLock},
};

use crate::{
  dds::{
    participant::{DomainParticipantWeak},
    typedesc::TypeDesc,
    qos::{
      QosPolicies, HasQoSPolicy,
      policy::{
        Reliability, History, Durability, Presentation, PresentationAccessScope, Deadline,
        Ownership, Liveliness, LivelinessKind, TimeBasedFilter, DestinationOrder, ResourceLimits,
      },
    },
    datareader::{DataReader},
    readcondition::ReadCondition,
    datawriter::DataWriter,
  },
  structure::guid::GUID,
  structure::entity::Entity,
};

use crate::discovery::{
  data_types::spdp_participant_data::SPDPDiscoveredParticipantData,
  data_types::topic_data::{DiscoveredWriterData, DiscoveredReaderData},
  discovery_db::DiscoveryDB,
};

use crate::structure::{duration::Duration, guid::EntityId};

use crate::serialization::{
  cdrSerializer::CDR_serializer_adapter, pl_cdr_deserializer::PlCdrDeserializerAdapter,
};

use crate::network::constant::*;
use super::data_types::topic_data::DiscoveredTopicData;
use byteorder::LittleEndian;

pub struct Discovery {
  poll: Poll,
  domain_participant: DomainParticipantWeak,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  discovery_updated_sender: mio_channel::Sender<DiscoveryNotificationType>,
  discovery_stop_receiver: mio_channel::Receiver<()>,
}

unsafe impl Sync for Discovery {}
unsafe impl Send for Discovery {}

impl Discovery {
  const PARTICIPANT_CLEANUP_PERIOD: u64 = 60;
  const TOPIC_CLEANUP_PERIOD: u64 = 5 * 60; // timer for cleaning up inactive topics
  const SEND_PARTICIPANT_INFO_PERIOD: u64 = 5;
  const SEND_READERS_INFO_PERIOD: u64 = 5;
  const SEND_WRITERS_INFO_PERIOD: u64 = 5;
  const SEND_TOPIC_INFO_PERIOD: u64 = 20;

  pub fn new(
    domain_participant: DomainParticipantWeak,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    discovery_updated_sender: mio_channel::Sender<DiscoveryNotificationType>,
    discovery_stop_receiver: mio_channel::Receiver<()>,
  ) -> Discovery {
    let poll = mio::Poll::new().expect("Unable to create discovery poll");

    Discovery {
      poll,
      domain_participant,
      discovery_db,
      discovery_updated_sender,
      discovery_stop_receiver,
    }
  }

  fn create_spdp_patricipant_qos() -> QosPolicies {
    let mut qos = QosPolicies::qos_none();
    qos.reliability = Some(Reliability::BestEffort);
    qos.history = Some(History::KeepLast { depth: 1 });
    qos
  }

  pub fn discovery_event_loop(discovery: Discovery) {
    discovery
      .poll
      .register(
        &discovery.discovery_stop_receiver,
        STOP_POLL_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register Discovery STOP");

    let discovery_subscriber_qos = Discovery::subscriber_qos();
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
    let dcps_participant_topic = discovery
      .domain_participant
      .create_topic(
        "DCPSParticipant",
        TypeDesc::new(String::from("SPDPDiscoveredParticipantData")),
        &Discovery::create_spdp_patricipant_qos(),
      )
      .expect("Unable to create DCPSParticipant topic.");

    let mut dcps_participant_reader = discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData,PlCdrDeserializerAdapter<SPDPDiscoveredParticipantData>>(
        Some(EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER),
        &dcps_participant_topic,
        &Discovery::create_spdp_patricipant_qos(),
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
      .create_datawriter::<SPDPDiscoveredParticipantData, CDR_serializer_adapter<SPDPDiscoveredParticipantData,LittleEndian> >(
        Some(EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER),
        &dcps_participant_topic,
        &Discovery::create_spdp_patricipant_qos(),
      )
      .expect("Unable to create DataWriter for DCPSParticipant.");
    discovery.initialize_participant(
      &discovery.domain_participant,
      dcps_participant_writer.get_guid(),
    );

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
    let dcps_subscription_qos = Discovery::subscriber_qos();
    let dcps_subscription_topic = discovery
      .domain_participant
      .create_topic(
        "DCPSSubscription",
        TypeDesc::new(String::from("DiscoveredReaderData")),
        &dcps_subscription_qos,
      )
      .expect("Unable to create DCPSSubscription topic.");

    let mut dcps_subscription_reader = discovery_subscriber
      .create_datareader::<DiscoveredReaderData, PlCdrDeserializerAdapter<DiscoveredReaderData>>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER),
        &dcps_subscription_topic,
        &Discovery::subscriber_qos(),
      )
      .expect("Unable to create DataReader for DCPSSubscription.");
    discovery
      .poll
      .register(
        &dcps_subscription_reader,
        DISCOVERY_READER_DATA_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to register subscription reader.");

    let mut dcps_subscription_writer = discovery_publisher
      .create_datawriter::<DiscoveredReaderData,CDR_serializer_adapter<DiscoveredReaderData,LittleEndian>>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER),
        &dcps_subscription_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataWriter for DCPSSubscription.");
    discovery.write_readers_info(&mut dcps_subscription_writer);

    let mut readers_send_info_timer: Timer<()> = Timer::default();
    readers_send_info_timer.set_timeout(
      StdDuration::from_secs(Discovery::SEND_READERS_INFO_PERIOD),
      (),
    );
    discovery
      .poll
      .register(
        &readers_send_info_timer,
        DISCOVERY_SEND_READERS_INFO_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to register readers info sender");

    // Publication
    let dcps_publication_qos = Discovery::subscriber_qos();
    let dcps_publication_topic = discovery
      .domain_participant
      .create_topic(
        "DCPSPublication",
        TypeDesc::new(String::from("DiscoveredWriterData")),
        &dcps_publication_qos,
      )
      .expect("Unable to create DCPSPublication topic.");

    let mut dcps_publication_reader = discovery_subscriber
      .create_datareader::<DiscoveredWriterData, PlCdrDeserializerAdapter<DiscoveredWriterData>>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER),
        &dcps_publication_topic,
        &Discovery::subscriber_qos(),
      )
      .expect("Unable to create DataReader for DCPSPublication");
    discovery
      .poll
      .register(
        &dcps_publication_reader,
        DISCOVERY_WRITER_DATA_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to regiser writers info sender.");

    let mut dcps_publication_writer = discovery_publisher
      .create_datawriter::<DiscoveredWriterData, CDR_serializer_adapter<DiscoveredWriterData,LittleEndian>>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER),
        &dcps_publication_topic,
        dcps_publication_topic.get_qos(),
      )
      .expect("Unable to create DataWriter for DCPSPublication.");
    discovery.write_writers_info(&mut dcps_publication_writer);

    let mut writers_send_info_timer: Timer<()> = Timer::default();
    writers_send_info_timer.set_timeout(
      StdDuration::from_secs(Discovery::SEND_WRITERS_INFO_PERIOD),
      (),
    );
    discovery
      .poll
      .register(
        &writers_send_info_timer,
        DISCOVERY_SEND_WRITERS_INFO_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to register readers info sender");

    // Topic
    let dcps_topic_qos = QosPolicies::qos_none();
    let dcps_topic = discovery
      .domain_participant
      .create_topic(
        "DCPSTopic",
        TypeDesc::new("DiscoveredTopicData".to_string()),
        &dcps_topic_qos,
      )
      .expect("Unable to create DCPSTopic topic.");
    // create lease duration check timer
    let mut topic_cleanup_timer: Timer<()> = Timer::default();
    topic_cleanup_timer.set_timeout(StdDuration::from_secs(Discovery::TOPIC_CLEANUP_PERIOD), ());
    discovery
      .poll
      .register(
        &topic_cleanup_timer,
        DISCOVERY_TOPIC_CLEANUP_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to register topic cleanup timer.");

    let mut dcps_reader = discovery_subscriber
      .create_datareader::<DiscoveredTopicData, PlCdrDeserializerAdapter<DiscoveredTopicData>>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER),
        &dcps_topic,
        &Discovery::subscriber_qos(),
      )
      .expect("Unable to create DataReader for DCPSTopic");
    discovery
      .poll
      .register(
        &dcps_reader,
        DISCOVERY_TOPIC_DATA_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to register topic reader");

    let mut dcps_writer = discovery_publisher
      .create_datawriter::<DiscoveredTopicData, CDR_serializer_adapter<DiscoveredTopicData,LittleEndian>>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER),
        &dcps_topic,
        &Discovery::subscriber_qos(),
      )
      .expect("Unable to create DataWriter for DCPSTopic.");
    let mut topic_info_send_timer: Timer<()> = Timer::default();
    topic_info_send_timer.set_timeout(
      StdDuration::from_secs(Discovery::SEND_TOPIC_INFO_PERIOD),
      (),
    );
    discovery
      .poll
      .register(
        &topic_info_send_timer,
        DISCOVERY_SEND_TOPIC_INFO_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to register topic info sender.");

    // waiting for init
    std::thread::sleep(StdDuration::from_secs(5));

    loop {
      let mut events = Events::with_capacity(1024);
      discovery
        .poll
        .poll(&mut events, None)
        .expect("Failed in waiting of poll.");

      for event in events.into_iter() {
        if event.token() == STOP_POLL_TOKEN {
          println!("Stopping Discovery");
          return;
        } else if event.token() == DISCOVERY_PARTICIPANT_DATA_TOKEN {
          let data = discovery.handle_participant_reader(&mut dcps_participant_reader);
          match data {
            Some(dat) => {
              discovery.update_spdp_participant_writer(dat);
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
          let strong_dp = match discovery.domain_participant.clone().upgrade() {
            Some(dp) => dp,
            None => {
              println!("DomainParticipant doesn't exist anymore, exiting Discovery.");
              return;
            }
          };
          let data = SPDPDiscoveredParticipantData::from_participant(&strong_dp, lease_duration);

          dcps_participant_writer.write(data, None).unwrap_or(());
          // reschedule timer
          participant_send_info_timer.set_timeout(
            StdDuration::from_secs(Discovery::SEND_PARTICIPANT_INFO_PERIOD),
            (),
          );
        } else if event.token() == DISCOVERY_READER_DATA_TOKEN {
          discovery.handle_subscription_reader(&mut dcps_subscription_reader);
        } else if event.token() == DISCOVERY_SEND_READERS_INFO_TOKEN {
          if discovery.read_readers_info() {
            discovery.write_readers_info(&mut dcps_subscription_writer);
          }

          readers_send_info_timer.set_timeout(
            StdDuration::from_secs(Discovery::SEND_READERS_INFO_PERIOD),
            (),
          );
        } else if event.token() == DISCOVERY_WRITER_DATA_TOKEN {
          discovery.handle_publication_reader(&mut dcps_publication_reader);
        } else if event.token() == DISCOVERY_SEND_WRITERS_INFO_TOKEN {
          if discovery.read_writers_info() {
            discovery.write_writers_info(&mut dcps_publication_writer);
          }

          writers_send_info_timer.set_timeout(
            StdDuration::from_secs(Discovery::SEND_WRITERS_INFO_PERIOD),
            (),
          );
        } else if event.token() == DISCOVERY_TOPIC_DATA_TOKEN {
          discovery.handle_topic_reader(&mut dcps_reader);
        } else if event.token() == DISCOVERY_TOPIC_CLEANUP_TOKEN {
          discovery.topic_cleanup();

          topic_cleanup_timer
            .set_timeout(StdDuration::from_secs(Discovery::TOPIC_CLEANUP_PERIOD), ());
        } else if event.token() == DISCOVERY_SEND_TOPIC_INFO_TOKEN {
          discovery.write_topic_info(&mut dcps_writer);
          topic_info_send_timer.set_timeout(
            StdDuration::from_secs(Discovery::SEND_TOPIC_INFO_PERIOD),
            (),
          );
        }
      }
    }
  }

  pub fn initialize_participant(&self, dp: &DomainParticipantWeak, writer_guid: GUID) {
    match self.discovery_db.write() {
      Ok(mut db) => {
        let port = get_spdp_well_known_multicast_port(dp.domain_id());
        db.initialize_participant_reader_proxy(writer_guid, port);
        self
          .discovery_updated_sender
          .send(DiscoveryNotificationType::WritersInfoUpdated)
          .unwrap();
      }
      _ => panic!("DiscoveryDB is poisoned."),
    }
  }

  pub fn handle_participant_reader(
    &self,
    reader: &mut DataReader<
      SPDPDiscoveredParticipantData,
      PlCdrDeserializerAdapter<SPDPDiscoveredParticipantData>,
    >,
  ) -> Option<SPDPDiscoveredParticipantData> {
    let participant_data = match reader.take_next_sample() {
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
          self
            .discovery_updated_sender
            .send(DiscoveryNotificationType::WritersInfoUpdated)
            .unwrap();
          self
            .discovery_updated_sender
            .send(DiscoveryNotificationType::ReadersInfoUpdated)
            .unwrap();
          return Some(participant_data);
        }
      }
      _ => return None,
    }

    None
  }

  pub fn handle_subscription_reader(
    &self,
    reader: &mut DataReader<DiscoveredReaderData, PlCdrDeserializerAdapter<DiscoveredReaderData>>,
  ) {
    let reader_data_vec: Option<Vec<DiscoveredReaderData>> =
      match reader.take(100, ReadCondition::not_read()) {
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
        let updated = p.update_subscription(data);
        if updated {
          match self
            .discovery_updated_sender
            .send(DiscoveryNotificationType::ReadersInfoUpdated)
          {
            Ok(_) => (),
            Err(_) => println!("Unable to update writers proxy."),
          };
          p.update_topic_data_drd(data);
        }
      }
    });
  }

  pub fn handle_publication_reader(
    &self,
    reader: &mut DataReader<DiscoveredWriterData, PlCdrDeserializerAdapter<DiscoveredWriterData>>,
  ) {
    let writer_data_vec: Option<Vec<DiscoveredWriterData>> =
      match reader.take(100, ReadCondition::not_read()) {
        Ok(d) => Some(
          d.into_iter()
            .map(|p| p.value)
            .filter(|p| p.is_ok())
            .map(|p| p.unwrap())
            .collect(),
        ),
        _ => None,
      };

    let writer_data_vec = match writer_data_vec {
      Some(d) => d,
      None => return,
    };

    match self.discovery_db.write().map(|mut p| {
      writer_data_vec.iter().for_each(|data| {
        let updated = p.update_publication(data);
        if updated {
          match self
            .discovery_updated_sender
            .send(DiscoveryNotificationType::ReadersInfoUpdated)
          {
            Ok(_) => (),
            Err(_) => println!("Unable to update readers proxy."),
          };
          p.update_topic_data_dwd(data);
        }
      })
    }) {
      Ok(_) => (),
      _ => panic!("DiscoveryDB is poisoned."),
    };
  }

  pub fn handle_topic_reader(
    &self,
    reader: &mut DataReader<DiscoveredTopicData, PlCdrDeserializerAdapter<DiscoveredTopicData>>,
  ) {
    let topic_data_vec: Option<Vec<DiscoveredTopicData>> =
      match reader.take(100, ReadCondition::any()) {
        Ok(d) => Some(
          d.into_iter()
            .map(|p| p.value)
            .filter(|p| p.is_ok())
            .map(|p| p.unwrap())
            .collect(),
        ),
        _ => None,
      };

    let topic_data_vec = match topic_data_vec {
      Some(d) => d,
      None => return,
    };

    match self.discovery_db.write().map(|mut p| {
      topic_data_vec.iter().for_each(|data| {
        let updated = p.update_topic_data(data);
        if updated {
          match self
            .discovery_updated_sender
            .send(DiscoveryNotificationType::TopicsInfoUpdated)
          {
            Ok(_) => (),
            Err(_) => println!("Unable to update topics proxy"),
          }
        }
      })
    }) {
      Ok(_) => (),
      _ => panic!("DiscoveryDB is poisoned"),
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

  pub fn topic_cleanup(&self) {
    match self.discovery_db.write() {
      Ok(mut db) => {
        db.topic_cleanup();
      }
      _ => return,
    }
  }

  pub fn update_spdp_participant_writer(&self, data: SPDPDiscoveredParticipantData) -> bool {
    let dbres = self.discovery_db.write();
    let res = match dbres {
      Ok(mut db) => (*db).update_participant(&data),
      _ => return false,
    };

    if res {
      match self
        .discovery_updated_sender
        .send(DiscoveryNotificationType::WritersInfoUpdated)
      {
        Ok(_) => (),
        _ => println!("Failed to send participant writer notification"),
      };
    }

    res
  }

  pub fn read_readers_info(&self) -> bool {
    let readers_info_updated = match self.discovery_db.read() {
      Ok(db) => db.is_readers_updated(),
      Err(e) => panic!("DiscoveryDB is poisoned {:?}", e),
    };

    if readers_info_updated {
      match self.discovery_db.write() {
        Ok(mut db) => db.readers_updated(false),
        Err(e) => panic!("DiscoveryDB is poisoned {:?}", e),
      }
    }

    readers_info_updated
  }

  pub fn read_writers_info(&self) -> bool {
    let writers_info_updated = match self.discovery_db.read() {
      Ok(db) => db.is_writers_updated(),
      Err(e) => panic!("DiscoveryDB is poisoned {:?}", e),
    };

    if writers_info_updated {
      match self.discovery_db.write() {
        Ok(mut db) => db.writers_updated(false),
        Err(e) => panic!("DiscoveryDB is poisoned {:?}", e),
      }
    }

    writers_info_updated
  }

  pub fn write_readers_info(
    &self,
    writer: &mut DataWriter<
      DiscoveredReaderData,
      CDR_serializer_adapter<DiscoveredReaderData, LittleEndian>,
    >,
  ) {
    match self.discovery_db.read() {
      Ok(db) => {
        let datas = db.get_all_local_topic_readers();
        for &data in datas
          .iter()
          // filtering out discoveries own readers
          .filter(|p| {
            let guid = match &p.reader_proxy.remote_reader_guid {
              Some(g) => g,
              None => return false,
            };
            let eid = &guid.entityId;

            *eid != EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER
              && *eid != EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER
              && *eid != EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER
              && *eid != EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER
          })
        {
          match writer.write(data.clone(), None) {
            Ok(_) => (),
            Err(e) => println!("Unable to write new readers info. {:?}", e),
          }
        }
      }
      _ => panic!("DiscoveryDB is poisoned."),
    }
  }

  pub fn write_writers_info(
    &self,
    writer: &mut DataWriter<
      DiscoveredWriterData,
      CDR_serializer_adapter<DiscoveredWriterData, LittleEndian>,
    >,
  ) {
    match self.discovery_db.read() {
      Ok(db) => {
        let datas = db.get_all_local_topic_writers();
        for &data in datas.iter().filter(|p| {
          let guid = match &p.writer_proxy.remote_writer_guid {
            Some(g) => g,
            None => return false,
          };
          let eid = &guid.entityId;

          *eid != EntityId::ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER
            && *eid != EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER
            && *eid != EntityId::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER
            && *eid != EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER
        }) {
          match writer.write(data.clone(), None) {
            Ok(_) => (),
            _ => println!("Unable to write new readers info."),
          }
        }
      }
      _ => panic!("DiscoveryDB is poisoned."),
    }
  }

  pub fn write_topic_info(
    &self,
    writer: &mut DataWriter<
      DiscoveredTopicData,
      CDR_serializer_adapter<DiscoveredTopicData, LittleEndian>,
    >,
  ) {
    match self.discovery_db.read() {
      Ok(db) => {
        let datas = db.get_all_topics();
        for &data in datas.iter() {
          match writer.write(data.clone(), None) {
            Ok(_) => (),
            _ => println!("Unable to write new topic info."),
          }
        }
      }
      _ => panic!("DiscoveryDB is poisoned."),
    }
  }

  pub fn subscriber_qos() -> QosPolicies {
    let mut qos = QosPolicies::qos_none();
    qos.durability = Some(Durability::TransientLocal);
    qos.presentation = Some(Presentation {
      access_scope: PresentationAccessScope::Topic,
      coherent_access: false,
      ordered_access: false,
    });
    qos.deadline = Some(Deadline {
      period: Duration::DURATION_INFINITE,
    });
    qos.ownership = Some(Ownership::Shared);
    qos.liveliness = Some(Liveliness {
      kind: LivelinessKind::Automatic,
      lease_duration: Duration::DURATION_INVALID,
    });
    qos.time_based_filter = Some(TimeBasedFilter {
      minimum_separation: Duration::DURATION_ZERO,
    });
    qos.reliability = Some(Reliability::Reliable {
      max_blocking_time: Duration::from(StdDuration::from_millis(100)),
    });
    qos.destination_order = Some(DestinationOrder::ByReceptionTimestamp);
    qos.history = Some(History::KeepLast { depth: 1 });
    qos.resource_limits = Some(ResourceLimits {
      max_instances: std::i32::MAX,
      max_samples: std::i32::MAX,
      max_samples_per_instance: std::i32::MAX,
    });
    qos
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    test::{
      shape_type::ShapeType,
      test_data::{
        spdp_subscription_msg, spdp_publication_msg, spdp_participant_msg_mod,
        create_rtps_data_message,
      },
    },
    network::{udp_listener::UDPListener, udp_sender::UDPSender},
    structure::{entity::Entity, locator::Locator},
    serialization::{cdrSerializer::to_bytes, cdrDeserializer::CDR_deserializer_adapter},
    submessages::{InterpreterSubmessage, EntitySubmessage},
    messages::{
      submessages::submessage_elements::serialized_payload::{RepresentationIdentifier},
    },
  };
  use crate::{
    discovery::data_types::topic_data::TopicBuiltinTopicData,
    dds::{participant::DomainParticipant, traits::serde_adapters::DeserializerAdapter},
  };
  use crate::serialization::submessage::*;

  use std::{time::Duration, net::SocketAddr};
  use mio::Token;
  use speedy::{Writable, Endianness};
  use byteorder::LittleEndian;

  #[test]
  fn discovery_participant_data_test() {
    let _participant = DomainParticipant::new(0, 0);

    let poll = Poll::new().unwrap();
    let mut udp_listener = UDPListener::new(Token(0), "127.0.0.1", 11000);
    poll
      .register(
        udp_listener.mio_socket(),
        Token(0),
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    // waiting for init
    std::thread::sleep(Duration::from_secs(5));

    // sending participant data to discovery
    let udp_sender = UDPSender::new_with_random_port();
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      get_spdp_well_known_unicast_port(0, 0),
    )];

    let tdata = spdp_participant_msg_mod(11000);
    let msg_data = tdata
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .expect("Failed to write msg data");

    udp_sender.send_to_all(&msg_data, &addresses);

    let mut events = Events::with_capacity(10);
    poll
      .poll(&mut events, Some(StdDuration::from_secs(10)))
      .unwrap();

    let _data2 = udp_listener.get_message();
    // TODO: we should have received our own participants info decoding the actual message might be good idea
  }

  #[test]
  fn discovery_reader_data_test() {
    let participant = DomainParticipant::new(14, 0);

    let topic = participant
      .create_topic(
        "Square",
        TypeDesc::new(String::from("ShapeType")),
        &QosPolicies::qos_none(),
      )
      .unwrap();

    let publisher = participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let _writer = publisher
      .create_datawriter::<ShapeType, CDR_serializer_adapter<ShapeType, LittleEndian>>(
        None,
        &topic,
        &QosPolicies::qos_none(),
      )
      .unwrap();

    let subscriber = participant
      .create_subscriber(&QosPolicies::qos_none())
      .unwrap();
    let _reader = subscriber.create_datareader::<ShapeType, CDR_deserializer_adapter<ShapeType>>(
      None,
      &topic,
      &QosPolicies::qos_none(),
    );

    let poll = Poll::new().unwrap();
    let mut udp_listener = UDPListener::new(Token(0), "127.0.0.1", 11001);
    poll
      .register(
        udp_listener.mio_socket(),
        Token(0),
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    // waiting for init
    std::thread::sleep(Duration::from_secs(5));

    let udp_sender = UDPSender::new_with_random_port();
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      get_spdp_well_known_unicast_port(14, 0),
    )];

    let mut tdata = spdp_subscription_msg();
    let mut data;
    for submsg in tdata.submessages.iter_mut() {
      match &mut submsg.body {
        SubmessageBody::Entity(v) => match v {
          EntitySubmessage::Data(d, _) => {
            let mut drd: DiscoveredReaderData = PlCdrDeserializerAdapter::from_bytes(
              &d.serialized_payload.value,
              RepresentationIdentifier::PL_CDR_LE,
            )
            .unwrap();
            drd.reader_proxy.unicast_locator_list.clear();
            drd
              .reader_proxy
              .unicast_locator_list
              .push(Locator::from(SocketAddr::new(
                "127.0.0.1".parse().unwrap(),
                11001,
              )));
            drd.reader_proxy.multicast_locator_list.clear();

            data = to_bytes::<DiscoveredReaderData, byteorder::LittleEndian>(&drd).unwrap();
            d.serialized_payload.value = data.clone();
          }
          _ => continue,
        },
        SubmessageBody::Interpreter(_) => (),
      }
    }

    let msg_data = tdata
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .expect("Failed to write msg dtaa");

    udp_sender.send_to_all(&msg_data, &addresses);

    let mut events = Events::with_capacity(10);
    poll
      .poll(&mut events, Some(StdDuration::from_secs(10)))
      .unwrap();

    let _data2 = udp_listener.get_message();
  }

  #[test]
  fn discovery_writer_data_test() {
    let participant = DomainParticipant::new(15, 0);

    let topic = participant
      .create_topic(
        "Square",
        TypeDesc::new(String::from("ShapeType")),
        &QosPolicies::qos_none(),
      )
      .unwrap();

    let publisher = participant
      .create_publisher(&QosPolicies::qos_none())
      .unwrap();
    let _writer = publisher
      .create_datawriter::<ShapeType, CDR_serializer_adapter<ShapeType, LittleEndian>>(
        None,
        &topic,
        &QosPolicies::qos_none(),
      )
      .unwrap();

    let subscriber = participant
      .create_subscriber(&QosPolicies::qos_none())
      .unwrap();
    let _reader = subscriber.create_datareader::<ShapeType, CDR_deserializer_adapter<ShapeType>>(
      None,
      &topic,
      &QosPolicies::qos_none(),
    );

    let poll = Poll::new().unwrap();
    let mut udp_listener = UDPListener::new(Token(0), "127.0.0.1", 11002);
    poll
      .register(
        udp_listener.mio_socket(),
        Token(0),
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    let udp_sender = UDPSender::new_with_random_port();
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      get_spdp_well_known_unicast_port(15, 0),
    )];

    let mut tdata = spdp_publication_msg();
    for submsg in tdata.submessages.iter_mut() {
      match &mut submsg.body {
        SubmessageBody::Interpreter(v) => match v {
          InterpreterSubmessage::InfoDestination(dst, _flags) => {
            dst.guid_prefix = participant.get_guid_prefix().clone();
          }
          _ => continue,
        },
        SubmessageBody::Entity(_) => (),
      }
    }

    let par_msg_data = spdp_participant_msg_mod(11002)
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .expect("Failed to write participant data.");

    let msg_data = tdata
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .expect("Failed to write msg data");

    // wait for init
    std::thread::sleep(Duration::from_secs(5));

    udp_sender.send_to_all(&par_msg_data, &addresses);
    udp_sender.send_to_all(&msg_data, &addresses);

    let mut events = Events::with_capacity(10);
    poll
      .poll(&mut events, Some(StdDuration::from_secs(10)))
      .unwrap();

    while !udp_listener.get_message().is_empty() {
      println!("Message received");
    }

    // small wait for stuff to happen
    std::thread::sleep(Duration::from_secs(2));
  }

  #[test]
  fn discovery_topic_data_test() {
    let _participant = DomainParticipant::new(16, 0);

    let topic_data = DiscoveredTopicData::new(TopicBuiltinTopicData {
      key: None,
      name: Some(String::from("Square")),
      type_name: Some(String::from("ShapeType")),
      durability: None,
      deadline: None,
      latency_budget: None,
      liveliness: None,
      reliability: None,
      lifespan: None,
      destination_order: None,
      presentation: None,
      history: None,
      resource_limits: None,
      ownership: None,
    });

    let rtps_message = create_rtps_data_message(
      topic_data,
      EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER,
      EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_WRITER,
    );

    let udp_sender = UDPSender::new_with_random_port();
    let addresses = vec![SocketAddr::new(
      "127.0.0.1".parse().unwrap(),
      get_spdp_well_known_unicast_port(16, 0),
    )];

    // wait for init
    std::thread::sleep(Duration::from_secs(5));

    let rr = rtps_message
      .write_to_vec_with_ctx(Endianness::LittleEndian)
      .unwrap();

    udp_sender.send_to_all(&rr, &addresses);

    // wait for stuff to happen
    std::thread::sleep(Duration::from_secs(5));
  }
}
