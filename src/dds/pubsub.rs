use mio_extras::channel as mio_channel;

use std::time::{Duration};

use serde::{Serialize, Deserialize};

use crate::structure::{guid::GUID, /*time::Timestamp,*/ entity::Entity, guid::EntityId};

use crate::dds::{
  values::result::*,
  participant::*,
  topic::*,
  qos::*,
  ddsdata::DDSData,
  reader::Reader,
  writer::Writer,
  datawriter::DataWriter,
  datareader::DataReader,
  traits::key::{Keyed, Key},
};

use crate::structure::topic_kind::TopicKind;

use rand::Rng;

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
    entity_id: Option<EntityId>,
    topic: &'a Topic,
    _qos: &QosPolicies,
  ) -> Result<DataWriter<'a, D>>
  where
    D: Keyed + Serialize,
    <D as Keyed>::K: Key,
  {
    let (dwcc_upload, hccc_download) = mio_channel::channel::<DDSData>();

    let entity_id = match entity_id {
      Some(eid) => eid,
      None => {
        let mut rng = rand::thread_rng();
        let eid = EntityId::createCustomEntityID([rng.gen(), rng.gen(), rng.gen()], 0xC2);

        eid
      }
    };

    let guid = GUID::new_with_prefix_and_id(
      self.get_participant().as_entity().guid.guidPrefix,
      entity_id,
    );
    let new_writer = Writer::new(
      guid,
      hccc_download,
      self.domain_participant.get_dds_cache(),
      topic.get_name().to_string(),
      topic.get_qos().clone(),
    );
    self
      .add_writer_sender
      .send(new_writer)
      .expect("Adding new writer failed");

    let matching_data_writer = DataWriter::<D>::new(
      self,
      &topic,
      dwcc_upload,
      self.get_participant().get_dds_cache(),
    );

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
  pub domain_participant: DomainParticipant,
  qos: QosPolicies,
  sender_add_reader: mio_channel::Sender<Reader>,
  sender_remove_reader: mio_channel::Sender<GUID>,
}

impl<'s> Subscriber {
  pub fn new(
    domainparticipant: DomainParticipant,
    qos: QosPolicies,
    sender_add_reader: mio_channel::Sender<Reader>,
    sender_remove_reader: mio_channel::Sender<GUID>,
  ) -> Subscriber {
    Subscriber {
      domain_participant: domainparticipant,
      qos,
      sender_add_reader,
      sender_remove_reader,
    }
  }

  pub fn create_datareader<D>(
    &'s self,
    entity_id: Option<EntityId>,
    topic: &'s Topic,
    _qos: &QosPolicies,
  ) -> Result<DataReader<'s, D>>
  where
    D: Deserialize<'s> + Keyed,
    <D as Keyed>::K: Key,
  {
    // What is the bound?
    let (send, rec) = mio_channel::sync_channel::<()>(10);

    let entity_id = match entity_id {
      Some(eid) => eid,
      None => {
        let mut rng = rand::thread_rng();
        let eid = EntityId::createCustomEntityID([rng.gen(), rng.gen(), rng.gen()], 0xC7);
        eid
      }
    };

    let reader_id = entity_id;
    let datareader_id = entity_id;

    let matching_datareader = DataReader::<D>::new(
      self,
      datareader_id,
      &topic,
      rec,
      self.domain_participant.get_dds_cache(),
    );

    let reader_guid =
      GUID::new_with_prefix_and_id(*self.domain_participant.get_guid_prefix(), reader_id);

    let new_reader = Reader::new(
      reader_guid,
      send,
      self.domain_participant.get_dds_cache(),
      topic.get_name().to_string(),
    );

    // Create new topic to DDScache if one isn't present
    self
      .domain_participant
      .get_dds_cache()
      .write()
      .unwrap()
      .add_new_topic(
        &topic.get_name().to_string(),
        TopicKind::NO_KEY,
        topic.get_type(),
      );

    // Return the DataReader Reader pairs to where they are used
    self
      .sender_add_reader
      .send(new_reader)
      .expect("Could not send new Reader");
    Ok(matching_datareader)
  }

  /// Retrieves a previously created DataReader belonging to the Subscriber.
  pub fn lookup_datareader<D: Keyed>(&self, _topic_name: &str) -> Option<DataReader<D>> {
    todo!()
  }
}

// -------------------------------------------------------------------

#[cfg(test)]
mod tests {}
