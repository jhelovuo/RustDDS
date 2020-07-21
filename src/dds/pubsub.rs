use std::time::Duration;
use mio_extras::channel as mio_channel;
use mio::{Ready, Registration, Poll, PollOpt, Token, SetReadiness, Event, Events};

use crate::network::constant::*;
use serde::{Serialize, Deserialize};

use crate::structure::time::Timestamp;
use crate::structure::guid::{GUID, EntityId};
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
use std::sync::Arc;
use std::io;

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

pub struct Subscriber<'a> {
  my_domainparticipant: &'a DomainParticipant,
  poll: Poll,
  qos: QosPolicies,
}

impl<'a> Subscriber<'a> {
  pub fn new(
    my_domainparticipant: &'a DomainParticipant,
    qos: QosPolicies, 
  ) -> Subscriber<'a>{
    let poll = Poll::new().expect("Unable to create new poll.");

    Subscriber{
      poll,
      my_domainparticipant,
      qos,
    }
  }

  pub fn subscriber_poll(mut subscriber: Subscriber) {
    loop {
      let mut events = Events::with_capacity(1024);

      subscriber.poll.poll(
        &mut events, None
      ).expect("Subscriber failed in polling");

      for event in events.into_iter() {
        println!("Subscriber poll received: {:?}", event); // for debugging!!!!!!

        match event.token() {
          STOP_POLL_TOKEN => return,
          READER_CHANGE_TOKEN => {
            println!("Got notification of reader's new change!!!");
          },
          _ => {},
        }
      }
    }
  }

  pub fn create_datareader<'p: 'a>(
    &'p self,
    _a_topic: &Topic,
    qos: QosPolicies,
  ) -> (Result<DataReader<'p>>, Result<Reader>) {

    let (register_datareader, 
      set_readiness_of_datareader) = Registration::new2();
    let (register_reader, 
      set_readiness_of_reader) = Registration::new2();

    let new_datareader = DataReader {
      my_subscriber: &self,
      qos_policy: qos,
      set_readiness: set_readiness_of_datareader,
      registration: register_datareader,
    };

    let matching_reader = Reader::new(
      GUID{
        guidPrefix: self.my_domainparticipant.get_guid_prefix(),
        entityId: EntityId::ENTITYID_PARTICIPANT,
      },
      set_readiness_of_reader,
      register_reader,
    );

    self.poll.register(
      &matching_reader,
      READER_CHANGE_TOKEN, 
      Ready::readable(),
       PollOpt::edge()
      ).expect("Failed to register reader with subscribers poll.");

    (Ok(new_datareader), Ok(matching_reader))
  }

}

// -------------------------------------------------------------------

pub struct DataReader<'s> {
  my_subscriber: &'s Subscriber<'s>,
  qos_policy: QosPolicies,
  set_readiness: SetReadiness,
  registration: Registration,
  // TODO: rest of fields
}

impl <'s> DataReader<'s> {
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

impl<'s> Evented for DataReader<'s> {
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

  #[test]
  fn sub_readers_notification() {
    let dp = DomainParticipant::new();
    let sub = dp.create_subsrciber(QosPolicies::qos_none()).unwrap();

    let (sender_stop, receiver_stop) = mio_channel::channel::<i32>();
    sub.poll.register(
      &receiver_stop, 
      STOP_POLL_TOKEN, 
      Ready::readable(), 
      PollOpt::edge()
    ).unwrap();

    let a_topic = Topic::new(
      &dp,
      ":D".to_string(),
      TypeDesc::new(":)".to_string()),
      QosPolicies::qos_none(),
    );
    let (dreader_res, reader_res) =
      sub.create_datareader(&a_topic, QosPolicies::qos_none());

    let mut reader = reader_res.unwrap();

    let child = thread::spawn(
      move || {
        std::thread::sleep(Duration::new(0,500));
        let d = Data::default();
        reader.handle_data_msg(d);
        std::thread::sleep(Duration::new(0,500));
        sender_stop.send(0).unwrap();
      }
    );
    Subscriber::subscriber_poll(sub);
    child.join().unwrap();
  }
}