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
  readcondition::ReadCondition,
  datawriter::DataWriter,
};

use crate::discovery::{
  data_types::spdp_participant_data::SPDPDiscoveredParticipantData,
  data_types::topic_data::{DiscoveredWriterData, DiscoveredReaderData},
  discovery_db::DiscoveryDB,
};

use crate::structure::guid::EntityId;

use crate::serialization::{
  cdrSerializer::CDR_serializer_adapter, pl_cdr_deserializer::PlCdrDeserializerAdapter,
};

use crate::network::constant::*;
use super::data_types::topic_data::DiscoveredTopicData;
use byteorder::LittleEndian;

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
  const SEND_PARTICIPANT_INFO_PERIOD: u64 = 2;
  const SEND_READERS_INFO_PERIOD: u64 = 1;
  const SEND_WRITERS_INFO_PERIOD: u64 = 1;

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
        TypeDesc::new(String::from("SPDPDiscoveredParticipantData")),
        &dcps_participant_qos,
      )
      .expect("Unable to create DCPSParticipant topic.");

    let mut dcps_participant_reader = discovery_subscriber
      .create_datareader::<SPDPDiscoveredParticipantData,PlCdrDeserializerAdapter<SPDPDiscoveredParticipantData>>(
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
      .create_datawriter::<SPDPDiscoveredParticipantData, CDR_serializer_adapter<SPDPDiscoveredParticipantData,LittleEndian> >(
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
        TypeDesc::new(String::from("DiscoveredReaderData")),
        &dcps_subscription_qos,
      )
      .expect("Unable to create DCPSSubscription topic.");

    let mut dcps_subscription_reader = discovery_subscriber
      .create_datareader::<DiscoveredReaderData, PlCdrDeserializerAdapter<DiscoveredReaderData>>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_READER),
        &dcps_subscription_topic,
        dcps_subscription_topic.get_qos(),
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
    let dcps_publication_qos = QosPolicies::qos_none();
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
        dcps_subscription_topic.get_qos(),
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
      .create_topic("DCPSTopic", TypeDesc::new("".to_string()), &dcps_topic_qos)
      .expect("Unable to create DCPSTopic topic.");
    let _dcps_reader = discovery_subscriber
      .create_datareader::<DiscoveredTopicData, PlCdrDeserializerAdapter<DiscoveredTopicData>>(
        Some(EntityId::ENTITYID_SEDP_BUILTIN_TOPIC_READER),
        &dcps_topic,
        dcps_subscription_topic.get_qos(),
      )
      .expect("Unable to create DataReader for DCPSTopic");
    let _dcps_writer = discovery_publisher
      .create_datawriter::<DiscoveredTopicData, CDR_serializer_adapter<DiscoveredTopicData,LittleEndian>>(
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
        } else if event.token() == DISCOVERY_READER_DATA_TOKEN {
          discovery.handle_subscription_reader(&mut dcps_subscription_reader);
        } else if event.token() == DISCOVERY_SEND_READERS_INFO_TOKEN {
          discovery.write_readers_info(&mut dcps_subscription_writer);

          readers_send_info_timer.set_timeout(
            StdDuration::from_secs(Discovery::SEND_READERS_INFO_PERIOD),
            (),
          );
        } else if event.token() == DISCOVERY_WRITER_DATA_TOKEN {
          discovery.handle_publication_reader(&mut dcps_publication_reader);
        } else if event.token() == DISCOVERY_SEND_WRITERS_INFO_TOKEN {
          discovery.write_writers_info(&mut dcps_publication_writer);

          writers_send_info_timer.set_timeout(
            StdDuration::from_secs(Discovery::SEND_WRITERS_INFO_PERIOD),
            (),
          );
        }
      }
    }
  }

  pub fn handle_participant_reader(
    &self,
    reader: &mut DataReader<
      SPDPDiscoveredParticipantData,
      PlCdrDeserializerAdapter<SPDPDiscoveredParticipantData>,
    >,
    //TODO: CDR is probably not what we want here. Change adapter to something else.
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
        let updated = (*p).update_subscription(data);
        if updated {
          match self.writers_proxy_updated_sender.send(()) {
            Ok(_) => (),
            Err(_) => println!("Unable to update writers proxy."),
          };
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
        let updated = (*p).update_publication(data);
        if updated {
          match self.readers_proxy_updated_sender.send(()) {
            Ok(_) => (),
            Err(_) => println!("Unable to update readers proxy."),
          };
        }
      })
    }) {
      Ok(_) => (),
      _ => panic!("DiscoveryDB is poisoned."),
    };
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

  pub fn update_spdp_participant_writer(&self, data: SPDPDiscoveredParticipantData) -> bool {
    let dbres = self.discovery_db.write();
    let res = match dbres {
      Ok(mut db) => (*db).update_participant(&data),
      _ => return false,
    };

    if res {
      match self.writers_proxy_updated_sender.send(()) {
        Ok(_) => (),
        _ => println!("Failed to send participant writer notification"),
      };
    }

    res
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
            _ => println!("Unable to write new readers info."),
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
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    test::{
      shape_type::ShapeType,
      test_data::{spdp_subscription_msg, spdp_publication_msg, spdp_participant_msg_mod},
    },
    network::{udp_listener::UDPListener, udp_sender::UDPSender},
    structure::{locator::Locator, entity::Entity},
    serialization::{cdrSerializer::to_bytes, cdrDeserializer::CDR_deserializer_adapter},
    submessages::{InterpreterSubmessage, EntitySubmessage},
    messages::submessages::submessage_elements::serialized_payload::RepresentationIdentifier,
  };
  use crate::dds::traits::serde_adapters::DeserializerAdapter;
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
      match submsg.submessage.as_mut() {
        Some(v) => match v {
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
        None => (),
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
      match submsg.intepreterSubmessage.as_mut() {
        Some(v) => match v {
          InterpreterSubmessage::InfoDestination(dst , _flags) => {
            dst.guid_prefix = participant.get_guid_prefix().clone();
          }
          _ => continue,
        },
        None => (),
      }
    }
    // TODO: modify message accordingly

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
}
