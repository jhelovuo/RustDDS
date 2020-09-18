use mio_extras::channel as mio_channel;

use std::{
  sync::{RwLock, Arc},
  time::Duration,
};

use serde::{Serialize, /*Deserialize,*/ de::DeserializeOwned};

use crate::{
  structure::{guid::GUID, /*time::Timestamp,*/ entity::Entity, guid::EntityId},
  discovery::discovery::DiscoveryCommand,
};

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
  traits::serde_adapters::*,
};

use crate::dds::no_key::datawriter as no_key_datawriter;

use crate::{
  discovery::{
    discovery_db::DiscoveryDB,
    data_types::topic_data::{DiscoveredWriterData},
  },
  structure::topic_kind::TopicKind,
};

use rand::Rng;

// -------------------------------------------------------------------

pub struct Publisher {
  domain_participant: DomainParticipantWeak,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  my_qos_policies: QosPolicies,
  default_datawriter_qos: QosPolicies, // used when creating a new DataWriter
  add_writer_sender: mio_channel::SyncSender<Writer>,
  discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
}

// public interface for Publisher
impl<'a> Publisher {
  pub fn new(
    dp: DomainParticipantWeak,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    qos: QosPolicies,
    default_dw_qos: QosPolicies,
    add_writer_sender: mio_channel::SyncSender<Writer>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  ) -> Publisher {
    Publisher {
      domain_participant: dp,
      discovery_db,
      my_qos_policies: qos,
      default_datawriter_qos: default_dw_qos,
      add_writer_sender,
      discovery_command,
    }
  }

  pub fn create_datawriter<D, SA>(
    &'a self,
    entity_id: Option<EntityId>,
    topic: &'a Topic,
    _qos: &QosPolicies,
  ) -> Result<DataWriter<'a, D, SA>>
  where
    D: Keyed + Serialize,
    <D as Keyed>::K: Key,
    SA: SerializerAdapter<D>,
  {
    let (dwcc_upload, hccc_download) = mio_channel::sync_channel::<DDSData>(100);

    let entity_id = match entity_id {
      Some(eid) => eid,
      None => {
        let mut rng = rand::thread_rng();
        let eid = EntityId::createCustomEntityID([rng.gen(), rng.gen(), rng.gen()], 0x02);

        eid
      }
    };

    let dp = match self.get_participant() {
      Some(dp) => dp,
      None => {
        println!("Cannot create new DataWriter, DomainParticipant doesn't exist.");
        return Err(Error::PreconditionNotMet);
      }
    };

    let guid = GUID::new_with_prefix_and_id(dp.as_entity().guid.guidPrefix, entity_id);
    let new_writer = Writer::new(
      guid.clone(),
      hccc_download,
      dp.get_dds_cache(),
      topic.get_name().to_string(),
      topic.get_qos().clone(),
    );

    self
      .add_writer_sender
      .send(new_writer)
      .expect("Adding new writer failed");

    let matching_data_writer = DataWriter::<D, SA>::new(
      self,
      &topic,
      Some(guid),
      dwcc_upload,
      self.discovery_command.clone(),
      dp.get_dds_cache(),
    );

    let matching_data_writer = match matching_data_writer {
      Ok(dw) => dw,
      e => return e,
    };

    match self.discovery_db.write() {
      Ok(mut db) => {
        let dwd = DiscoveredWriterData::new(&matching_data_writer, &topic, &dp);

        db.update_local_topic_writer(dwd);
        db.update_topic_data_p(&topic);
      }
      _ => return Err(Error::OutOfResources),
    };

    Ok(matching_data_writer)
  }

  pub fn create_datawriter_no_key<D, SA>(
    &'a self,
    entity_id: Option<EntityId>,
    topic: &'a Topic,
    qos: &QosPolicies,
  ) -> Result<no_key_datawriter::DataWriter<'a, D, SA>>
  where
    D: Serialize,
    SA: SerializerAdapter<D>,
  {
    let d = self.create_datawriter::
      <no_key_datawriter::NoKeyWrapper_Write<D>,no_key_datawriter::SA_Wrapper<SA>>
        (entity_id, topic, qos)?;
    Ok(no_key_datawriter::DataWriter::<'a, D, SA>::from_keyed(d))
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
  pub fn get_participant(&self) -> Option<DomainParticipant> {
    self.domain_participant.clone().upgrade()
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
  domain_participant: DomainParticipantWeak,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  qos: QosPolicies,
  sender_add_reader: mio_channel::SyncSender<Reader>,
  sender_remove_reader: mio_channel::SyncSender<GUID>,
  discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
}

impl<'s> Subscriber {
  pub fn new(
    domain_participant: DomainParticipantWeak,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    qos: QosPolicies,
    sender_add_reader: mio_channel::SyncSender<Reader>,
    sender_remove_reader: mio_channel::SyncSender<GUID>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  ) -> Subscriber {
    Subscriber {
      domain_participant,
      discovery_db,
      qos,
      sender_add_reader,
      sender_remove_reader,
      discovery_command,
    }
  }

  pub fn create_datareader<D, SA>(
    &'s self,
    entity_id: Option<EntityId>,
    topic: &'s Topic,
    _qos: &QosPolicies,
  ) -> Result<DataReader<'s, D, SA>>
  where
    D: DeserializeOwned + Keyed,
    <D as Keyed>::K: Key,
    SA: DeserializerAdapter<D>,
  {
    // What is the bound?
    let (send, rec) = mio_channel::sync_channel::<()>(10);

    let entity_id = match entity_id {
      Some(eid) => eid,
      None => {
        let mut rng = rand::thread_rng();
        let eid = EntityId::createCustomEntityID([rng.gen(), rng.gen(), rng.gen()], 0x07);
        eid
      }
    };

    let reader_id = entity_id;
    let datareader_id = entity_id;

    let dp = match self.get_participant() {
      Some(dp) => dp,
      None => {
        println!("DomainParticipant doesn't exist anymore.");
        return Err(Error::PreconditionNotMet);
      }
    };

    let matching_datareader = DataReader::<D, SA>::new(
      self,
      datareader_id,
      &topic,
      rec,
      dp.get_dds_cache(),
      self.discovery_command.clone(),
    );

    let matching_datareader = match matching_datareader {
      Ok(dr) => dr,
      e => return e,
    };

    let reader_guid = GUID::new_with_prefix_and_id(dp.get_guid_prefix(), reader_id);

    let new_reader = Reader::new(
      reader_guid,
      send,
      dp.get_dds_cache(),
      topic.get_name().to_string(),
    );

    match self.discovery_db.write() {
      Ok(mut db) => {
        db.update_local_topic_reader(&dp, &topic, &new_reader);
        db.update_topic_data_p(&topic);
      }
      _ => return Err(Error::OutOfResources),
    };

    // Create new topic to DDScache if one isn't present
    match dp.get_dds_cache().write() {
      Ok(mut rwlock) => rwlock.add_new_topic(
        &topic.get_name().to_string(),
        TopicKind::WITH_KEY,
        topic.get_type(),
      ),
      Err(e) => panic!(
        "The DDSCache of domain participant {:?} is poisoned. Error: {}",
        dp.get_guid(),
        e
      ),
    };

    // Return the DataReader Reader pairs to where they are used
    self
      .sender_add_reader
      .try_send(new_reader)
      .expect("Could not send new Reader");
    Ok(matching_datareader)
  }

  /// Retrieves a previously created DataReader belonging to the Subscriber.
  pub fn lookup_datareader<D, SA>(&self, _topic_name: &str) -> Option<DataReader<D, SA>>
  where
    D: Keyed + DeserializeOwned,
    SA: DeserializerAdapter<D>,
  {
    todo!()
    // TO think: Is this really necessary? Because the caller would have to know
    // types D and SA. Sould we just trust whoever creates DataReaders to also remember them?
  }

  pub fn get_participant(&self) -> Option<DomainParticipant> {
    self.domain_participant.clone().upgrade()
  }
}

// -------------------------------------------------------------------

#[cfg(test)]
mod tests {}
