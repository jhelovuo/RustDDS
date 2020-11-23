use mio_extras::channel as mio_channel;
use log::error;

use std::{fmt::Debug, sync::{RwLock, Arc}, time::Duration};

use serde::{Serialize, de::DeserializeOwned};

use crate::{
  discovery::discovery::DiscoveryCommand,
  structure::{guid::GUID, entity::Entity, guid::EntityId},
};

use crate::dds::{
  values::result::*,
  participant::*,
  topic::*,
  qos::*,
  reader::Reader,
  writer::Writer,
  with_key::datawriter::DataWriter as WithKeyDataWriter,
  no_key::datawriter::DataWriter as NoKeyDataWriter,
  with_key::datareader::DataReader as WithKeyDataReader,
  no_key::datareader::DataReader as NoKeyDataReader,
  traits::key::{Keyed, Key},
  traits::serde_adapters::*,
};

use crate::{
  discovery::{
    discovery_db::DiscoveryDB,
    data_types::topic_data::{DiscoveredWriterData},
  },
  structure::topic_kind::TopicKind,
};

use rand::Rng;

use super::{
  with_key::datareader::ReaderCommand,
  no_key::{wrappers::NoKeyWrapper, wrappers::SAWrapper},
  writer::WriterCommand,
};

// -------------------------------------------------------------------

/// DDS Publisher
#[derive(Clone)]
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
  pub(super) fn new(
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

  /// Creates DDS [DataWriter](struct.With_Key_DataWriter.html) for Keyed topic
  ///
  /// # Arguments
  ///
  /// * `entity_id` - Custom entity id if necessary for the user to define it
  /// * `topic` - Reference to DDS Topic this writer is created to
  /// * `qos` - Not currently in use
  pub fn create_datawriter<D, SA>(
    &'a self,
    entity_id: Option<EntityId>,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<WithKeyDataWriter<'a, D, SA>>
  where
    D: Keyed + Serialize,
    <D as Keyed>::K: Key,
    SA: SerializerAdapter<D>,
  {
    let (dwcc_upload, hccc_download) = mio_channel::sync_channel::<WriterCommand>(100);
    let (message_status_sender, message_status_receiver) = mio_channel::sync_channel(100);

    // TODO: check compatible qos and use QOS
    let _qos = match qos {
      Some(q) => q.clone(),
      None => topic.get_qos().clone(),
    };

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
        error!("Cannot create new DataWriter, DomainParticipant doesn't exist.");
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
      message_status_sender,
    );

    self
      .add_writer_sender
      .send(new_writer)
      .expect("Adding new writer failed");

    let matching_data_writer = WithKeyDataWriter::<D, SA>::new(
      self,
      &topic,
      Some(guid),
      dwcc_upload,
      self.discovery_command.clone(),
      dp.get_dds_cache(),
      message_status_receiver,
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

  /// Creates DDS [DataWriter](struct.DataWriter.html) for Nokey Topic
  ///
  /// # Arguments
  ///
  /// * `entity_id` - Custom entity id if necessary for the user to define it
  /// * `topic` - Reference to DDS Topic this writer is created to
  /// * `qos` - Not currently in use
  pub fn create_datawriter_no_key<D, SA>(
    &'a self,
    entity_id: Option<EntityId>,
    topic: &'a Topic,
    qos: Option<QosPolicies>,
  ) -> Result<NoKeyDataWriter<'a, D, SA>>
  where
    D: Serialize,
    SA: SerializerAdapter<D>,
  {
    let entity_id = match entity_id {
      Some(eid) => eid,
      None => {
        let mut rng = rand::thread_rng();
        let eid = EntityId::createCustomEntityID([rng.gen(), rng.gen(), rng.gen()], 0x03);

        eid
      }
    };
    let d =
      self.create_datawriter::<NoKeyWrapper<D>, SAWrapper<SA>>(Some(entity_id), topic, qos)?;
    Ok(NoKeyDataWriter::<'a, D, SA>::from_keyed(d))
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
  /// Currently does nothing
  pub fn suspend_publications(&self) -> Result<()> {
    Ok(())
  }

  /// Currently does nothing
  pub fn resume_publications(&self) -> Result<()> {
    Ok(())
  }

  // coherent change set
  // In case such QoS is not supported, these should be no-ops.
  // TODO: Implement these when coherent change-sets are supported.
  /// Coherent set not implemented and currently does nothing
  pub fn begin_coherent_changes(&self) -> Result<()> {
    Ok(())
  }

  /// Coherent set not implemented and currently does nothing
  pub fn end_coherent_changes(&self) -> Result<()> {
    Ok(())
  }

  // Wait for all matched reliable DataReaders acknowledge data written so far, or timeout.
  // TODO: implement
  pub(crate) fn wait_for_acknowledgments(&self, _max_wait: Duration) -> Result<()> {
    unimplemented!();
  }

  // What is the use case for this? (is it useful in Rust style of programming? Should it be public?)
  /// Gets [DomainParticipant](struct.DomainParticipant.html) if it has not disappeared from all scopes.
  pub fn get_participant(&self) -> Option<DomainParticipant> {
    self.domain_participant.clone().upgrade()
  }

  // delete_contained_entities: We should not need this. Contained DataWriters should dispose themselves and notify publisher.

  /// Returns default DataWriter qos. Currently default qos is not used.
  pub fn get_default_datawriter_qos(&self) -> &QosPolicies {
    &self.default_datawriter_qos
  }

  /// Sets default DataWriter qos. Currenly default qos is not used.
  pub fn set_default_datawriter_qos(&mut self, q: &QosPolicies) {
    self.default_datawriter_qos = q.clone();
  }
}

impl PartialEq for Publisher {
  fn eq(&self, other: &Self) -> bool {
    self.get_participant() == other.get_participant() &&
    self.my_qos_policies == other.my_qos_policies &&
    self.default_datawriter_qos == other.default_datawriter_qos
    // TODO: publisher is DDSEntity?
  }
}

impl Debug for Publisher {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("{:?}", self.get_participant()))?;
    f.write_fmt(format_args!("Publisher QoS: {:?}", self.my_qos_policies))?;
    f.write_fmt(format_args!("Publishers default Writer QoS: {:?}", self.default_datawriter_qos))
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

/// DDS Subscriber
#[derive(Clone)]
pub struct Subscriber {
  domain_participant: DomainParticipantWeak,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  qos: QosPolicies,
  sender_add_reader: mio_channel::SyncSender<Reader>,
  sender_remove_reader: mio_channel::SyncSender<GUID>,
  discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
}

impl<'s> Subscriber {
  pub(super) fn new(
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

  /*pub(super)*/
  fn create_datareader_internal<D: 'static, SA>(
    &'s self,
    entity_id: Option<EntityId>,
    topic: &'s Topic,
    //topic_kind: Option<TopicKind>,
    qos: Option<QosPolicies>,
  ) -> Result<WithKeyDataReader<'s, D, SA>>
  where
    D: DeserializeOwned + Keyed,
    <D as Keyed>::K: Key,
    SA: DeserializerAdapter<D>,
  {
    // What is the bound?
    let (send, rec) = mio_channel::sync_channel::<()>(10);
    let (status_sender, status_receiver) = mio_channel::sync_channel::<StatusChange>(10);
    let (reader_command_sender, reader_command_receiver) =
      mio_channel::sync_channel::<ReaderCommand>(10);

    // TODO: use qos
    let _qos = match qos {
      Some(q) => q,
      None => topic.get_qos().clone(),
    };

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
        error!("DomainParticipant doesn't exist anymore.");
        return Err(Error::PreconditionNotMet);
      }
    };

    let reader_guid = GUID::new_with_prefix_and_id(dp.get_guid_prefix(), reader_id);

    let new_reader = Reader::new(
      reader_guid,
      send,
      status_sender,
      dp.get_dds_cache(),
      topic.get_name().to_string(),
      reader_command_receiver,
    );

    let matching_datareader = WithKeyDataReader::<D, SA>::new(
      self,
      datareader_id,
      &topic,
      rec,
      dp.get_dds_cache(),
      self.discovery_command.clone(),
      status_receiver,
      reader_command_sender,
    );

    let matching_datareader = match matching_datareader {
      Ok(dr) => dr,
      e => return e,
    };

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
        topic.topic_kind,
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

  /// Creates DDS DataReader for keyed Topics
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to the DDS [Topic](struct.Topic.html) this reader reads from
  /// * `entity_id` - Optional [EntityId](data_types/struct.EntityId.html) if necessary for DDS communication (random if None)
  /// * `qos` - Not in use  
  pub fn create_datareader<D: 'static, SA>(
    &'s self,
    topic: &'s Topic,
    entity_id: Option<EntityId>,
    qos: Option<QosPolicies>,
  ) -> Result<WithKeyDataReader<'s, D, SA>>
  where
    D: DeserializeOwned + Keyed,
    <D as Keyed>::K: Key,
    SA: DeserializerAdapter<D>,
  {
    if topic.topic_kind != TopicKind::WithKey {
      return Err(Error::PreconditionNotMet); // TopicKind mismatch
    }
    self.create_datareader_internal(entity_id, topic, qos)
  }

  /// Create DDS DataReader for non keyed Topics
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to the DDS [Topic](struct.Topic.html) this reader reads from
  /// * `entity_id` - Optional [EntityId](data_types/struct.EntityId.html) if necessary for DDS communication (random if None)
  /// * `qos` - Not in use  
  pub fn create_datareader_no_key<D: 'static, SA>(
    &'s self,
    topic: &'s Topic,
    entity_id: Option<EntityId>,
    qos: Option<QosPolicies>,
  ) -> Result<NoKeyDataReader<'s, D, SA>>
  where
    D: DeserializeOwned,
    SA: DeserializerAdapter<D>,
  {
    if topic.topic_kind != TopicKind::NoKey {
      return Err(Error::PreconditionNotMet); // TopicKind mismatch
    }

    let entity_id = match entity_id {
      Some(eid) => eid,
      None => {
        let mut rng = rand::thread_rng();
        let eid = EntityId::createCustomEntityID([rng.gen(), rng.gen(), rng.gen()], 0x04);
        eid
      }
    };

    let d = self.create_datareader_internal::<NoKeyWrapper<D>, SAWrapper<SA>>(
      Some(entity_id),
      topic,
      qos,
    )?;

    Ok(NoKeyDataReader::<'s, D, SA>::from_keyed(d))
  }

  /// Retrieves a previously created DataReader belonging to the Subscriber.
  // TODO: Is this even possible. Whould probably need to return reference and store references on creation
  pub(crate) fn lookup_datareader<D, SA>(
    &self,
    _topic_name: &str,
  ) -> Option<WithKeyDataReader<D, SA>>
  where
    D: Keyed + DeserializeOwned,
    SA: DeserializerAdapter<D>,
  {
    todo!()
    // TO think: Is this really necessary? Because the caller would have to know
    // types D and SA. Sould we just trust whoever creates DataReaders to also remember them?
  }

  /// Returns [DomainParticipant](struct.DomainParticipant.html) if it is sill alive.
  pub fn get_participant(&self) -> Option<DomainParticipant> {
    self.domain_participant.clone().upgrade()
  }
}

// -------------------------------------------------------------------

#[cfg(test)]
mod tests {}
