use std::time::Duration;
use mio::{Ready, Registration, Poll, PollOpt, Token, SetReadiness, Events};

use crate::network::constant::*;
use serde::{Serialize, Deserialize};

use crate::structure::time::Timestamp;
use crate::structure::guid::{GUID, EntityId};
use mio_extras::channel as mio_channel;
use crate::structure::entity::{Entity, EntityAttributes};

use crate::dds::result::*;
use crate::dds::participant::*;
use crate::dds::topic::*;
use crate::dds::key::*;
use crate::dds::typedesc::*;
use crate::dds::qos::*;
use crate::dds::datasample::*;
use crate::dds::reader::Reader;

use mio::event::Evented;
use std::io;

use std::sync::{Arc, Mutex};
use crate::structure::history_cache::HistoryCache;

// -------------------------------------------------------------------

pub struct Publisher<'a> {
  my_domainparticipant: &'a DomainParticipant,
  my_qos_policies: QosPolicies,
  default_datawriter_qos: QosPolicies, // used when creating a new DataWriter
}

// public interface for Publisher
impl<'a> Publisher<'a> {
  pub fn create_datawriter<'p, D>(
    &self,
    _a_topic: &Topic,
    _qos: QosPolicies,
  ) -> Result<DataWriter<'p>> {
    unimplemented!();
  }

  // delete_datawriter should not be needed. The DataWriter object itself should be deleted to accomplish this.

  // lookup datawriter: maybe not necessary? App should remember datawriters it has created.

  // Suspend and resume publications are preformance optimization methods.
  // The minimal correct implementation is to do nothing. See DDS spec 2.2.2.4.1.8 and .9
  pub fn suspend_publications(&self) -> Result<()> {
    Ok(())
  }
  pub fn resume_publications(&self) -> Result<()> {
    Ok(())
  }

  // coherent change set
  // In case such QoS is not supported, these should be no-ops.
  // TODO: Implement these when coherent change-sets are supported.
  pub fn begin_coherent_changes(&self) -> Result<()> {
    Ok(())
  }
  pub fn end_coherent_changes(&self) -> Result<()> {
    Ok(())
  }

  // Wait for all matched reliable DataReaders acknowledge data written so far, or timeout.
  pub fn wait_for_acknowledgments(&self, _max_wait: Duration) -> Result<()> {
    unimplemented!();
  }

  // What is the use case for this? (is it useful in Rust style of programming? Should it be public?)
  pub fn get_participant(&self) -> &DomainParticipant {
    self.my_domainparticipant
  }

  // delete_contained_entities: We should not need this. Contained DataWriters should dispose themselves and notify publisher.

  pub fn get_default_datawriter_qos(&self) -> QosPolicies {
    self.default_datawriter_qos.clone()
  }
  pub fn set_default_datawriter_qos(&mut self, q: QosPolicies) {
    self.default_datawriter_qos = q;
  }
}

// -------------------------------------------------------------------

pub struct Subscriber {
  //my_domainparticipant: &'a DomainParticipant,
  poll: Poll,
  qos: QosPolicies,
  datareaders: Vec<DataReader>,

  sender_add_reader: mio_channel::Sender<Reader>,
  sender_remove_reader:  mio_channel::Sender<GUID>,

  receiver_remove_datareader: mio_channel::Receiver<GUID>,
  participant_guid: GUID,
}

impl Subscriber {
  pub fn new(
    //my_domainparticipant: &'a DomainParticipant,
    qos: QosPolicies, 
    sender_add_reader: mio_channel::Sender<Reader>,
    sender_remove_reader:  mio_channel::Sender<GUID>,

    receiver_add_datareader: mio_channel::Receiver<()>,
    receiver_remove_datareader: mio_channel::Receiver<GUID>,

    participant_guid: GUID,
  ) -> Subscriber{
    let poll = Poll::new().expect("Unable to create new poll.");

    poll.register(
      &receiver_add_datareader,
      ADD_DATAREADER_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ).expect("Failed to register datareader adder.");

    poll.register(
      &receiver_remove_datareader,
      REMOVE_DATAREADER_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    ).expect("Failed to register datareader remover.");

    Subscriber{
      poll,
      //my_domainparticipant,
      qos,
      datareaders: Vec::new(),
      sender_add_reader,
      sender_remove_reader,
      receiver_remove_datareader,
      participant_guid,
    }
  }

  pub fn subscriber_poll(&mut self) {
    loop {
      println!("Subscriber looping...");
      let mut events = Events::with_capacity(1024);

      self.poll.poll(
        &mut events, None
      ).expect("Subscriber failed in polling");

      for event in events.into_iter() {
        println!("Subscriber poll received: {:?}", event); // for debugging!!!!!!

        match event.token() {
          STOP_POLL_TOKEN => return,
          READER_CHANGE_TOKEN => {
            println!("Got notification of reader's new change!!!");
            println!("Should get information on who the reader was???, 
                      and ask for the change?");
          },
          ADD_DATAREADER_TOKEN => {
            let (dr, r) = 
              self.create_datareader(self.participant_guid, self.qos.clone());

            let reader = r.unwrap();
            self.poll.register(
              &reader,
              READER_CHANGE_TOKEN, 
              Ready::readable(),
              PollOpt::edge(),
            ).expect("Unable to register new Reader for Subscribers poll.");
            self.datareaders.push(dr.unwrap());
            self.sender_add_reader.send(reader).unwrap();
          },
          REMOVE_DATAREADER_TOKEN => {
            let old_dr_guid = self.receiver_remove_datareader.try_recv().unwrap();
            if let Some(pos) = self.datareaders.iter().position(
              |r| r.get_guid() == old_dr_guid
            ) {
                self.datareaders.remove(pos);
            }
            self.sender_remove_reader.send(old_dr_guid).unwrap();
          },
          _ => {},
        }
      }
    }
  }

  pub fn create_datareader(
    &self,
    participant_guid: GUID,
    qos: QosPolicies,
  ) -> (Result<DataReader>, Result<Reader>) {

    let (register_datareader, 
      set_readiness_of_datareader) = Registration::new2();


    let history_cache = 
    Arc::new(Mutex::new(HistoryCache::new()));

    let new_datareader = DataReader {
      //my_subscriber: &self,
      qos_policy: qos,
      set_readiness: set_readiness_of_datareader,
      registration: register_datareader,
      history_cache: history_cache.clone(),
      entity_attributes: EntityAttributes{guid: participant_guid},
    };

    let matching_reader = Reader::new(
      participant_guid,
      history_cache,
    );

    (Ok(new_datareader), Ok(matching_reader))
  }

}

// -------------------------------------------------------------------

pub struct DataReader {
  //my_subscriber: &'s Subscriber<'s>,
  qos_policy: QosPolicies,
  set_readiness: SetReadiness,
  registration: Registration,
  history_cache: Arc<Mutex<HistoryCache>>,
  entity_attributes: EntityAttributes,
  // TODO: rest of fields
}

impl <'s> DataReader {
  pub fn read<D>(&self, _max_samples: i32) -> Result<Vec<DataSample<D>>>
  where D: Deserialize<'s> + Keyed, 
  { 
    unimplemented!() 
  }

  pub fn take<D>(&self, _max_samples: i32) -> Result<Vec<DataSample<D>>>
  where D: Deserialize<'s> + Keyed, 
  { 
    unimplemented!() 
  }


} // impl 

impl Entity for DataReader {
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
  }
}

impl Evented for DataReader {
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.registration.register(poll, token, interest, opts)
  }
  fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.registration.reregister(poll, token, interest, opts)
  }
  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.registration.deregister(poll)
  }
}

pub struct DataWriter<'p> {
  my_publisher: &'p Publisher<'p>,
  my_topic: &'p Topic<'p>,
}

impl<'p> DataWriter<'p> {
  // Instance registration operations:
  // * register_instance (_with_timestamp)
  // * unregister_instance (_with_timestamp)
  // * get_key_value  (InstanceHandle --> Key)
  // * lookup_instance (Key --> InstanceHandle)
  // Do not implement these until there is a clear use case for InstanceHandle type.

  // write (with optional timestamp)
  // This operation could take also in InstanceHandle, if we would use them.
  // The _with_timestamp version is covered by the optional timestamp.
  pub fn write<D>(&self, _data: D, _source_timestamp: Option<Timestamp>)
  where
    D: Serialize + Keyed,
  {
  }

  // dispose
  // The data item is given only for identification, i.e. extracting the key
  pub fn dispose<D>(&self, _data: &D, _source_timestamp: Option<Timestamp>)
  where
    D: Serialize + Keyed,
  {
  }

  pub fn wait_for_acknowledgments(&self, _max_wait: Duration) -> Result<()> {
    unimplemented!();
  }

  // status queries
  pub fn get_liveliness_lost_status(&self) -> Result<LivelinessLostStatus> {
    unimplemented!()
  }
  pub fn get_offered_deadline_missed_status(&self) -> Result<OfferedDeadlineMissedStatus> {
    unimplemented!()
  }
  pub fn get_offered_incompatibel_qos_status(&self) -> Result<OfferedIncompatibelQosStatus> {
    unimplemented!()
  }
  pub fn get_publication_matched_status(&self) -> Result<PublicationMatchedStatus> {
    unimplemented!()
  }

  // who are we connected to?
  pub fn get_topic(&self) -> &Topic {
    unimplemented!()
  }
  pub fn get_publisher(&self) -> &Publisher {
    self.my_publisher
  }

  pub fn assert_liveliness(&self) -> Result<()> {
    unimplemented!()
  }

  // This shoudl really return InstanceHandles pointing to a BuiltInTopic reader
  //  but let's see if we can do without those handles.
  pub fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData> {
    unimplemented!()
  }
  // This one function provides both get_matched_subscrptions and get_matched_subscription_data
  // TODO: Maybe we could return references to the subscription data to avoid copying?
  // But then what if the result set changes while the application processes it?
}


#[cfg(test)]
mod tests{
  use super::*;
  use crate::dds::topic::Topic;
  use crate::dds::typedesc::TypeDesc;
  
  use std::thread;
  use std::time::Duration;
  use crate::messages::submessages::data::Data;
  use crate::messages::submessages::heartbeat::Heartbeat;
  use crate::structure::sequence_number::SequenceNumber;
  use mio_extras::channel as mio_channel;
  use std::sync::{Arc, Mutex};
  use crate::structure::history_cache::HistoryCache;

  #[test]
  fn sub_subpoll_test() {
    let dp_guid = GUID::new();

    let (sender_add_datareader, receiver_add_datareader) =
    mio_channel::channel::<()>();
    let (sender_remove_datareader, receiver_remove_datareader) =
    mio_channel::channel::<GUID>();

    let (sender_add_reader, receiver_add_reader) =
    mio_channel::channel::<Reader>();
    let (sender_remove_reader, receiver_remove_reader) =
    mio_channel::channel::<GUID>();

    let mut sub = Subscriber::new(
      QosPolicies::qos_none(),
      sender_add_reader.clone(),
      sender_remove_reader.clone(),

      receiver_add_datareader,
      receiver_remove_datareader,

      dp_guid,
    );

    let (sender_stop, receiver_stop) = mio_channel::channel::<i32>();
    sub.poll.register(
      &receiver_stop, 
      STOP_POLL_TOKEN, 
      Ready::readable(), 
      PollOpt::edge()
    ).unwrap();

    let (dr, r) = 
    sub.create_datareader(sub.participant_guid, sub.qos.clone());

    let mut reader = r.unwrap();
    sub.poll.register(
      &reader,
      READER_CHANGE_TOKEN, 
      Ready::readable(),
      PollOpt::edge(),
    ).expect("Unable to register new Reader for Subscribers poll.");
    sub.datareaders.push(dr.unwrap());

    let child = thread::spawn(
      move || {
        std::thread::sleep(Duration::new(0,500));
        let d = Data::default();
        reader.handle_data_msg(d);

        std::thread::sleep(Duration::new(0,500_000));
        sender_stop.send(0).unwrap();
      }
    );

    sub.subscriber_poll();
    child.join().unwrap();



  }
}