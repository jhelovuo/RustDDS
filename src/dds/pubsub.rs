use std::time::Duration;
use mio::{Ready, Poll, PollOpt, Events};

use crate::network::constant::*;

use crate::structure::guid::{GUID};
use crate::structure::time::Timestamp;

use mio_extras::channel as mio_channel;
use crate::structure::entity::{Entity};

use crate::dds::values::result::*;
use crate::dds::participant::*;
use crate::dds::topic::*;
use crate::dds::qos::*;
use crate::dds::ddsdata::DDSData;
use crate::dds::reader::Reader;
use crate::dds::writer::Writer;
use crate::dds::datawriter::DataWriter;
use crate::dds::datareader::DataReader;

use rand::Rng;
use crate::structure::guid::EntityId;

// -------------------------------------------------------------------

pub struct Publisher<'a> {
  my_domainparticipant: &'a DomainParticipant,
  my_qos_policies: QosPolicies,
  default_datawriter_qos: QosPolicies, // used when creating a new DataWriter
  add_writer_sender: mio_channel::Sender<Writer>,
}

// public interface for Publisher
impl<'a> Publisher<'a> {
  pub fn new(
    dp: &'a DomainParticipant,
    qos: QosPolicies,
    default_dw_qos: QosPolicies,
    add_writer_sender: mio_channel::Sender<Writer>,
  ) -> Publisher {
    Publisher {
      my_domainparticipant: dp,
      my_qos_policies: qos,
      default_datawriter_qos: default_dw_qos,
      add_writer_sender,
    }
  }

  pub fn create_datawriter(&'a self, topic: &'a Topic, _qos: QosPolicies) -> Result<DataWriter> {
    let (dwcc_upload, hccc_download) = mio_channel::channel::<DDSData>();

    // TODO: generate unique entity id's in a more systematic way
    let mut rng = rand::thread_rng();
    let entity_id = EntityId::createCustomEntityID([rng.gen(), rng.gen(), rng.gen()], 0xC2);

    let guid = GUID::new_with_prefix_and_id(
      self.get_participant().as_entity().guid.guidPrefix,
      entity_id,
    );
    let new_writer = Writer::new(guid, hccc_download);
    self
      .add_writer_sender
      .send(new_writer)
      .expect("Adding new writer failed");

    let matching_data_writer = DataWriter::new(&self, topic, dwcc_upload);

    Ok(matching_data_writer)
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
// -------------------------------------------------------------------
// -------------------------------------------------------------------
// -------------------------------------------------------------------
// -------------------------------------------------------------------
// -------------------------------------------------------------------
// -------------------------------------------------------------------
// -------------------------------------------------------------------
// -------------------------------------------------------------------
// -------------------------------------------------------------------

pub struct Subscriber {
  //my_domainparticipant: &'a DomainParticipant,
  poll: Poll,
  qos: QosPolicies,
  datareaders: Vec<DataReader>,

  sender_add_reader: mio_channel::Sender<Reader>,
  sender_remove_reader: mio_channel::Sender<GUID>,

  receiver_remove_datareader: mio_channel::Receiver<GUID>,

  reader_channel_ends: Vec<(EntityId, mio_channel::Receiver<(DDSData, Timestamp)>)>,
  participant_guid: GUID,
}

impl Subscriber {
  pub fn new(
    //my_domainparticipant: &'a DomainParticipant,
    qos: QosPolicies,
    sender_add_reader: mio_channel::Sender<Reader>,
    sender_remove_reader: mio_channel::Sender<GUID>,

    receiver_add_datareader: mio_channel::Receiver<()>,
    receiver_remove_datareader: mio_channel::Receiver<GUID>,

    participant_guid: GUID,
  ) -> Subscriber {
    let poll = Poll::new().expect("Unable to create new poll.");

    poll
      .register(
        &receiver_add_datareader,
        ADD_DATAREADER_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register datareader adder.");

    poll
      .register(
        &receiver_remove_datareader,
        REMOVE_DATAREADER_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Failed to register datareader remover.");

    Subscriber {
      poll,
      //my_domainparticipant,
      qos,
      datareaders: Vec::new(),
      sender_add_reader,
      sender_remove_reader,
      receiver_remove_datareader,
      reader_channel_ends: Vec::new(),
      participant_guid,
    }
  }

  pub fn subscriber_poll(&mut self) {
    loop {
      println!("Subscriber looping...");
      let mut events = Events::with_capacity(1024);

      self
        .poll
        .poll(&mut events, None)
        .expect("Subscriber failed in polling");

      for event in events.into_iter() {
        println!("Subscriber poll received: {:?}", event); // for debugging!!!!!!

        match event.token() {
          STOP_POLL_TOKEN => return,
          READER_CHANGE_TOKEN => {
            // Eti oikee datareader
            for pos in 0..(self.reader_channel_ends.len() as usize) {
              let _channel_message = self.reader_channel_ends[pos].1.try_recv();
              //match channel_message
            }
          }
          ADD_DATAREADER_TOKEN => {
            let dr = self.create_datareader(self.participant_guid, self.qos.clone());

            self.datareaders.push(dr.unwrap());
          }
          REMOVE_DATAREADER_TOKEN => {
            let old_dr_guid = self.receiver_remove_datareader.try_recv().unwrap();
            if let Some(pos) = self
              .datareaders
              .iter()
              .position(|r| r.get_guid() == old_dr_guid)
            {
              self.datareaders.remove(pos);
            }
            self.sender_remove_reader.send(old_dr_guid).unwrap();
          }
          _ => {}
        }
      }
    }
  }

  pub fn create_datareader(
    &mut self,
    participant_guid: GUID,
    qos: QosPolicies,
  ) -> Result<DataReader> {
    let new_datareader = DataReader::new(qos);

    let (send, rec) = mio_channel::channel::<(DDSData, Timestamp)>();
    let matching_reader = Reader::new(participant_guid, send);

    self
      .poll
      .register(
        &rec,
        READER_CHANGE_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("Unable to register new Reader for Subscribers poll.");

    self
      .reader_channel_ends
      .push((matching_reader.get_entity_id(), rec));
    self.sender_add_reader.send(matching_reader).unwrap();
    Ok(new_datareader)
  }

  pub fn lookup_datareader(&self, _topic_name: String) -> Option<Vec<&DataReader>> {
    todo!()
  }
}

// -------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use std::thread;
  use std::time::Duration;
  use crate::messages::submessages::data::Data;
  use mio_extras::channel as mio_channel;
  use crate::dds::message_receiver::MessageReceiverInfo;

  #[test]
  fn sub_subpoll_test() {
    let dp_guid = GUID::new();

    let (_sender_add_datareader, receiver_add_datareader) = mio_channel::channel::<()>();
    let (_sender_remove_datareader, receiver_remove_datareader) = mio_channel::channel::<GUID>();

    let (sender_add_reader, receiver_add_reader) = mio_channel::channel::<Reader>();
    let (sender_remove_reader, _receiver_remove_reader) = mio_channel::channel::<GUID>();

    let mut sub = Subscriber::new(
      QosPolicies::qos_none(),
      sender_add_reader,
      sender_remove_reader,
      receiver_add_datareader,
      receiver_remove_datareader,
      dp_guid,
    );

    let (sender_stop, receiver_stop) = mio_channel::channel::<i32>();
    sub
      .poll
      .register(
        &receiver_stop,
        STOP_POLL_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    let dr = sub.create_datareader(sub.participant_guid, sub.qos.clone());
    
    
    let mut reader = receiver_add_reader.try_recv().unwrap();
    sub.datareaders.push(dr.unwrap());

    let child = thread::spawn(move || {
      std::thread::sleep(Duration::new(0, 500));
      let d = Data::default();
      reader.handle_data_msg(d, MessageReceiverInfo::default());

      std::thread::sleep(Duration::new(0, 500));
      let d2 = Data::default();
      reader.handle_data_msg(d2, MessageReceiverInfo::default());

      std::thread::sleep(Duration::new(0, 500_000));
      sender_stop.send(0).unwrap();
    });
    sub.subscriber_poll();

    match child.join() {
      _ => {}
    }
  }
}
