use mio::{Ready, Poll, PollOpt};
use mio_extras::channel as mio_channel;

use crate::network::constant::*;

use std::{time::Duration, sync::Arc, rc::Rc};

use rand::Rng;
use serde::Deserialize;

use crate::structure::{guid::GUID, time::Timestamp, entity::Entity, guid::EntityId};

use crate::dds::{
  values::result::*, participant::*, topic::*, qos::*, ddsdata::DDSData, reader::Reader,
  writer::Writer, datawriter::DataWriter, datareader::DataReader,
  traits::datasample_trait::DataSampleTrait,
};

// -------------------------------------------------------------------

pub struct Publisher {
  domain_participant: DomainParticipant,
  my_qos_policies: QosPolicies,
  default_datawriter_qos: QosPolicies, // used when creating a new DataWriter
  add_writer_sender: mio_channel::Sender<Writer>,
}

// public interface for Publisher
impl<'a> Publisher {
  pub fn new(
    dp: DomainParticipant,
    qos: QosPolicies,
    default_dw_qos: QosPolicies,
    add_writer_sender: mio_channel::Sender<Writer>,
  ) -> Publisher {
    Publisher {
      domain_participant: dp,
      my_qos_policies: qos,
      default_datawriter_qos: default_dw_qos,
      add_writer_sender,
    }
  }

  pub fn create_datawriter<D>(
    &'a self,
    topic: &'a Topic,
    _qos: &QosPolicies,
  ) -> Result<DataWriter<'a, D>>
  where
    D: DataSampleTrait,
  {
    let (dwcc_upload, hccc_download) = mio_channel::channel::<DDSData>();

    // TODO: generate unique entity id's in a more systematic way
    let mut rng = rand::thread_rng();
    let entity_id = EntityId::createCustomEntityID([rng.gen(), rng.gen(), rng.gen()], 0xC2);

    let guid = GUID::new_with_prefix_and_id(
      self.get_participant().as_entity().guid.guidPrefix,
      entity_id,
    );
    let new_writer = Writer::new(
      guid,
      hccc_download,
      self.domain_participant.get_dds_cache(),
      topic.get_name().to_string(),
    );
    self
      .add_writer_sender
      .send(new_writer)
      .expect("Adding new writer failed");

    let matching_data_writer = DataWriter::<D>::new(self, &topic, dwcc_upload);

    Ok(matching_data_writer)
  }

  fn add_writer(&self, writer: Writer) -> Result<()> {
    match self.add_writer_sender.send(writer) {
      Ok(_) => Ok(()),
      _ => Err(Error::OutOfResources),
    }
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
    &self.domain_participant
  }

  // delete_contained_entities: We should not need this. Contained DataWriters should dispose themselves and notify publisher.

  pub fn get_default_datawriter_qos(&self) -> &QosPolicies {
    &self.default_datawriter_qos
  }
  pub fn set_default_datawriter_qos(&mut self, q: &QosPolicies) {
    self.default_datawriter_qos = q.clone();
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
  domain_participant: DomainParticipant,
  qos: QosPolicies,
  sender_add_reader: mio_channel::Sender<Reader>,
  sender_remove_reader: mio_channel::Sender<GUID>,
}

impl<'s> Subscriber {
  pub fn new(domain_participant: &DomainParticipant, qos: &QosPolicies) -> Subscriber {
    let sender_add_reader = domain_participant.get_add_reader_sender();
    let sender_remove_reader = domain_participant.get_remove_reader_sender();
    Subscriber {
      domain_participant: domain_participant.clone(),
      qos,
      sender_add_reader,
      sender_remove_reader,
    }
  }

  pub fn create_datareader<D>(
    &self,
    participant_guid: &GUID,
    qos: &QosPolicies,
  ) -> Result<DataReader<D>>
  where
    D: Deserialize<'s> + DataSampleTrait,
  {
    // TODO: generate unique entity id's in a more systematic way

    let mut rng = rand::thread_rng();
    let entity_id = EntityId::createCustomEntityID([rng.gen(), rng.gen(), rng.gen()], 0xC7);
    let guid = GUID::new_with_prefix_and_id(
      self.domain_participant.as_entity().guid.guidPrefix,
      entity_id,
    );

    let (send, rec) = mio_channel::channel::<(DDSData, Timestamp)>();
    let matching_reader = Reader::new(&participant_guid, send);
    self.domain_participant.add_reader(matching_reader);

    let new_datareader = DataReader::<D>::new(&guid, &self, &qos, rec);

    Ok(new_datareader)
  }
}

// -------------------------------------------------------------------

#[cfg(test)]
mod tests {}
