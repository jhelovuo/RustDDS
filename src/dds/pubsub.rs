use mio_extras::channel as mio_channel;
use log::error;

use std::{
  fmt::Debug,
  sync::{RwLock, Arc},
  time::Duration,
};

use serde::{Serialize, de::DeserializeOwned};

use byteorder::{LittleEndian};

use crate::{
  discovery::discovery::DiscoveryCommand,
  structure::{guid::GUID, entity::RTPSEntity, guid::EntityId},
};

use crate::log_and_err_precondition_not_met;
use crate::dds::{
  values::result::*,
  data_types::EntityKind,
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
    data_types::topic_data::DiscoveredWriterData,
  },
  structure::topic_kind::TopicKind,
  serialization::cdr_serializer::{CDRSerializerAdapter},
  serialization::cdr_deserializer::{CDRDeserializerAdapter},
};

use rand::Rng;
use crate::log_and_err_internal;

use super::{
  with_key::datareader::ReaderCommand,
  no_key::{wrappers::NoKeyWrapper, wrappers::SAWrapper},
  writer::WriterCommand,
};

// -------------------------------------------------------------------

/// DDS Publisher
///
/// # Examples
///
/// ```
/// # use rustdds::dds::DomainParticipant;
/// # use rustdds::dds::qos::QosPolicyBuilder;
/// use rustdds::dds::Publisher;
///
/// let domain_participant = DomainParticipant::new(0);
/// let qos = QosPolicyBuilder::new().build();
///
/// let publisher = domain_participant.create_publisher(&qos);
/// ```
#[derive(Clone)]
pub struct Publisher {
  inner: Arc<InnerPublisher>,
}

impl Publisher {
  pub(super) fn new(
    dp: DomainParticipantWeak,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    qos: QosPolicies,
    default_dw_qos: QosPolicies,
    add_writer_sender: mio_channel::SyncSender<Writer>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  ) -> Publisher {
    Publisher {
      inner: Arc::new(InnerPublisher::new(dp,discovery_db,qos,default_dw_qos,add_writer_sender,discovery_command) )
    }
  }

  /// Creates DDS [DataWriter](struct.With_Key_DataWriter.html) for Keyed topic
  ///
  /// # Arguments
  ///
  /// * `entity_id` - Custom entity id if necessary for the user to define it
  /// * `topic` - Reference to DDS Topic this writer is created to
  /// * `qos` - Not currently in use
  ///
  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::Publisher;
  /// # use rustdds::dds::data_types::TopicKind;
  /// use rustdds::dds::traits::Keyed;
  /// use rustdds::serialization::CDRSerializerAdapter;
  /// use serde::Serialize;
  ///
  /// let domain_participant = DomainParticipant::new(0);
  /// let qos = QosPolicyBuilder::new().build();
  ///
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn get_key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// let topic = domain_participant.create_topic("some_topic", "SomeType", &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(None, topic, None);
  /// ```
  pub fn create_datawriter<D, SA>(
    &self,
    entity_id: Option<EntityId>,
    topic: Topic,
    qos: Option<QosPolicies>,
  ) -> Result<WithKeyDataWriter<D, SA>>
  where
    D: Keyed + Serialize,
    <D as Keyed>::K: Key,
    SA: SerializerAdapter<D>,
  {
    self.inner.create_datawriter(self, entity_id, topic, qos)
  }

  /// Shorthand for crate_datawriter with Commaon Data Representation Little Endian
  pub fn create_datawriter_CDR<D>(&self, entity_id: Option<EntityId>, 
      topic: Topic, qos: Option<QosPolicies>) 
    -> Result<WithKeyDataWriter<D, CDRSerializerAdapter<D,LittleEndian>>>
  where
    D: Keyed + Serialize,
    <D as Keyed>::K: Key,
  {
    self.create_datawriter::<D,CDRSerializerAdapter<D,LittleEndian>>(entity_id,topic,qos)
  }

  /// Creates DDS [DataWriter](struct.DataWriter.html) for Nokey Topic
  ///
  /// # Arguments
  ///
  /// * `entity_id` - Custom entity id if necessary for the user to define it
  /// * `topic` - Reference to DDS Topic this writer is created to
  /// * `qos` - QoS policies for this DataWriter
  ///
  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::Publisher;
  /// # use rustdds::dds::data_types::TopicKind;
  /// use rustdds::serialization::CDRSerializerAdapter;
  /// use serde::Serialize;
  ///
  /// let domain_participant = DomainParticipant::new(0);
  /// let qos = QosPolicyBuilder::new().build();
  ///
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize)]
  /// struct SomeType {}
  ///
  /// let topic = domain_participant.create_topic("some_topic", "SomeType", &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter_no_key::<SomeType, CDRSerializerAdapter<_>>(None, topic, None);
  /// ```
  pub fn create_datawriter_no_key<D, SA>(
    &self,
    entity_id: Option<EntityId>,
    topic: Topic,
    qos: Option<QosPolicies>,
  ) -> Result<NoKeyDataWriter<D, SA>>
  where
    D: Serialize,
    SA: SerializerAdapter<D>,
  {
    self.inner.create_datawriter_no_key(self, entity_id,topic,qos)
  }

  pub fn create_datawriter_no_key_CDR<D>(&self, entity_id: Option<EntityId>, 
      topic: Topic, qos: Option<QosPolicies>) 
    -> Result<NoKeyDataWriter<D,CDRSerializerAdapter<D,LittleEndian> >>
  where
    D: Serialize,
  {
    self.create_datawriter_no_key::<D,CDRSerializerAdapter<D,LittleEndian>>(entity_id,topic,qos)
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
  ///
  /// # Example
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::Publisher;
  ///
  /// let domain_participant = DomainParticipant::new(0);
  /// let qos = QosPolicyBuilder::new().build();
  ///
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  /// assert_eq!(domain_participant, publisher.get_participant().unwrap());
  /// ```
  pub fn get_participant(&self) -> Option<DomainParticipant> {
    self.inner.domain_participant.clone().upgrade()
  }

  // delete_contained_entities: We should not need this. Contained DataWriters should dispose themselves and notify publisher.

  /// Returns default DataWriter qos. Currently default qos is not used.
  ///
  /// # Example
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// use rustdds::dds::qos::{QosPolicyBuilder};
  /// # use rustdds::dds::Publisher;
  ///
  /// let domain_participant = DomainParticipant::new(0);
  /// let qos = QosPolicyBuilder::new().build();
  ///
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  /// assert_eq!(qos, publisher.get_default_datawriter_qos());
  /// ```
  pub fn get_default_datawriter_qos(&self) -> QosPolicies {
    self.inner.default_datawriter_qos.clone()
  }

  // / Sets default DataWriter qos. Currenly default qos is not used.
  // /
  // / # Example
  // /
  // / ```
  // / # use rustdds::dds::DomainParticipant;
  // / # use rustdds::dds::qos::{QosPolicyBuilder, policy::Durability};
  // / # use rustdds::dds::Publisher;
  // /
  // / let domain_participant = DomainParticipant::new(0);
  // / let qos = QosPolicyBuilder::new().build();
  // /
  // / let mut publisher = domain_participant.create_publisher(&qos).unwrap();
  // / let qos2 = QosPolicyBuilder::new().durability(Durability::Transient).build();
  // / publisher.set_default_datawriter_qos(&qos2);
  // /
  // / assert_ne!(qos, *publisher.get_default_datawriter_qos());
  // / assert_eq!(qos2, *publisher.get_default_datawriter_qos());
  // / ```
  // pub fn set_default_datawriter_qos(&mut self, q: &QosPolicies) {
  //   self.inner.borrow_mut().default_datawriter_qos = q.clone();
  // }
} // impl

impl PartialEq for Publisher {
  fn eq(&self, other: &Self) -> bool {
    self.inner == other.inner  // use Eq implementation of Rc
  }
}

impl Debug for Publisher {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.inner.fmt(f)
  }
}

// "Inner" struct

#[derive(Clone)]
struct InnerPublisher {
  domain_participant: DomainParticipantWeak,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  my_qos_policies: QosPolicies,
  default_datawriter_qos: QosPolicies, // used when creating a new DataWriter
  add_writer_sender: mio_channel::SyncSender<Writer>,
  discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
}

// public interface for Publisher
impl InnerPublisher {
  fn new(
    dp: DomainParticipantWeak,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    qos: QosPolicies,
    default_dw_qos: QosPolicies,
    add_writer_sender: mio_channel::SyncSender<Writer>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  ) -> InnerPublisher {
    InnerPublisher {
      domain_participant: dp,
      discovery_db,
      my_qos_policies: qos,
      default_datawriter_qos: default_dw_qos,
      add_writer_sender,
      discovery_command,
    }
  }


  pub fn create_datawriter<D, SA>(
    &self,
    outer: &Publisher,
    entity_id_opt: Option<EntityId>,
    topic: Topic,
    optional_qos: Option<QosPolicies>,
  ) -> Result<WithKeyDataWriter<D, SA>>
  where
    D: Keyed + Serialize,
    <D as Keyed>::K: Key,
    SA: SerializerAdapter<D>,
  {
    let (dwcc_upload, hccc_download) = mio_channel::sync_channel::<WriterCommand>(100);
    let (message_status_sender, message_status_receiver) = mio_channel::sync_channel(100);

   
    // DDS Spec 2.2.2.4.1.5 create_datawriter:
    // If no QoS is specified, we should take the Publisher default
    // QoS, modify it to match any QoS settings (that are set) in the
    // Topic QoS and use that.
    let writer_qos = optional_qos.unwrap_or_else(
        || self.default_datawriter_qos.modify_by(&topic.get_qos()) );

    let entity_id = unwrap_or_random_EntityId(entity_id_opt, EntityKind::WRITER_WITH_KEY_USER_DEFINED);
    let dp = self.get_participant()
              .ok_or("upgrade fail")
              .or_else (|e| log_and_err_internal!("Where is my DomainParticipant? {}",e))?;

    let guid = GUID::new_with_prefix_and_id(dp.get_guid().guidPrefix, entity_id);
    let new_writer = Writer::new(
      guid.clone(),
      hccc_download,
      dp.get_dds_cache(),
      topic.get_name().to_string(),
      writer_qos,
      message_status_sender,
    );

    self.add_writer_sender.send(new_writer)
      .or_else(|e| log_and_err_internal!("Adding new writer failed: {}",e))?;

    let matching_data_writer = WithKeyDataWriter::<D, SA>::new(
          outer.clone(),
          topic.clone(),
          Some(guid),
          dwcc_upload,
          self.discovery_command.clone(),
          dp.get_dds_cache(),
          message_status_receiver,
        )?;

    // notify Discovery DB
    let mut db = self.discovery_db.write()?;
    let dwd = DiscoveredWriterData::new(&matching_data_writer, &topic, &dp);
    db.update_local_topic_writer(dwd);
    db.update_topic_data_p(&topic);

    Ok(matching_data_writer)
  }

  pub fn create_datawriter_no_key<D, SA>(
    &self,
    outer: &Publisher,
    entity_id_opt: Option<EntityId>,
    topic: Topic,
    qos: Option<QosPolicies>,
  ) -> Result<NoKeyDataWriter<D, SA>>
  where
    D: Serialize,
    SA: SerializerAdapter<D>,
  {
    let entity_id = unwrap_or_random_EntityId(entity_id_opt, EntityKind::WRITER_NO_KEY_USER_DEFINED);
    let d =
      self.create_datawriter::<NoKeyWrapper<D>, SAWrapper<SA>>(outer, Some(entity_id), topic, qos)?;
    Ok(NoKeyDataWriter::<D, SA>::from_keyed(d))
  }

  fn add_writer(&self, writer: Writer) -> Result<()> {
    match self.add_writer_sender.send(writer) {
      Ok(_) => Ok(()),
      _ => Err(Error::OutOfResources),
    }
  }

  pub fn suspend_publications(&self) -> Result<()> {
    Ok(())
  }

  pub fn resume_publications(&self) -> Result<()> {
    Ok(())
  }

  pub fn begin_coherent_changes(&self) -> Result<()> {
    Ok(())
  }

  pub fn end_coherent_changes(&self) -> Result<()> {
    Ok(())
  }

  pub(crate) fn wait_for_acknowledgments(&self, _max_wait: Duration) -> Result<()> {
    unimplemented!();
  }

  pub fn get_participant(&self) -> Option<DomainParticipant> {
    self.domain_participant.clone().upgrade()
  }

  pub fn get_default_datawriter_qos(&self) -> &QosPolicies {
    &self.default_datawriter_qos
  }

  pub fn set_default_datawriter_qos(&mut self, q: &QosPolicies) {
    self.default_datawriter_qos = q.clone();
  }
}

impl PartialEq for InnerPublisher {
  fn eq(&self, other: &Self) -> bool {
    self.get_participant() == other.get_participant()
      && self.my_qos_policies == other.my_qos_policies
      && self.default_datawriter_qos == other.default_datawriter_qos
    // TODO: publisher is DDSEntity?
  }
}

impl Debug for InnerPublisher {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("{:?}", self.get_participant()))?;
    f.write_fmt(format_args!("Publisher QoS: {:?}", self.my_qos_policies))?;
    f.write_fmt(format_args!(
      "Publishers default Writer QoS: {:?}",
      self.default_datawriter_qos
    ))
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
///
/// # Examples
///
/// ```
/// # use rustdds::dds::DomainParticipant;
/// # use rustdds::dds::qos::QosPolicyBuilder;
/// use rustdds::dds::Subscriber;
///
/// let domain_participant = DomainParticipant::new(0);
/// let qos = QosPolicyBuilder::new().build();
///
/// let subscriber = domain_participant.create_subscriber(&qos);
/// ```
#[derive(Clone)]
pub struct Subscriber {
  inner: Arc<InnerSubscriber>,
}

impl Subscriber {
  pub(super) fn new(
    domain_participant: DomainParticipantWeak,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    qos: QosPolicies,
    sender_add_reader: mio_channel::SyncSender<Reader>,
    sender_remove_reader: mio_channel::SyncSender<GUID>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  ) -> Subscriber {
    Subscriber {
      inner: Arc::new(
        InnerSubscriber::new(domain_participant, discovery_db, qos, 
                sender_add_reader, sender_remove_reader, discovery_command))
    }
  }

  /// Creates DDS DataReader for keyed Topics
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to the DDS [Topic](struct.Topic.html) this reader reads from
  /// * `entity_id` - Optional [EntityId](data_types/struct.EntityId.html) if necessary for DDS communication (random if None)
  /// * `qos` - Not in use
  ///
  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::Subscriber;
  /// use serde::Deserialize;
  /// use rustdds::serialization::CDRDeserializerAdapter;
  /// use rustdds::dds::data_types::TopicKind;
  /// use rustdds::dds::traits::Keyed;
  /// #
  /// # let domain_participant = DomainParticipant::new(0);
  /// # let qos = QosPolicyBuilder::new().build();
  /// #
  ///
  /// let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  ///
  /// #[derive(Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn get_key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// let topic = domain_participant.create_topic("some_topic", "SomeType", &qos, TopicKind::WithKey).unwrap();
  /// let data_reader = subscriber.create_datareader::<SomeType, CDRDeserializerAdapter<_>>(topic, None, None);
  /// ```
  pub fn create_datareader<D: 'static, SA>(
    &self,
    topic: Topic,
    entity_id: Option<EntityId>,
    qos: Option<QosPolicies>,
  ) -> Result<WithKeyDataReader<D, SA>>
  where
    D: DeserializeOwned + Keyed,
    <D as Keyed>::K: Key,
    SA: DeserializerAdapter<D>,
  {
    self.inner
      .create_datareader(self,topic,entity_id,qos)
  }

  pub fn create_datareader_CDR<D: 'static>(
    &self,
    topic: Topic,
    entity_id: Option<EntityId>,
    qos: Option<QosPolicies>,
  ) -> Result<WithKeyDataReader<D, CDRDeserializerAdapter<D>>>
  where
    D: DeserializeOwned + Keyed,
    <D as Keyed>::K: Key,
  {
    self.create_datareader::<D,CDRDeserializerAdapter<D>>(topic,entity_id,qos)
  }

  /// Create DDS DataReader for non keyed Topics
  ///
  /// # Arguments
  ///
  /// * `topic` - Reference to the DDS [Topic](struct.Topic.html) this reader reads from
  /// * `entity_id` - Optional [EntityId](data_types/struct.EntityId.html) if necessary for DDS communication (random if None)
  /// * `qos` - Not in use  
  ///
  /// # Examples
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::Subscriber;
  /// use serde::Deserialize;
  /// use rustdds::serialization::CDRDeserializerAdapter;
  /// use rustdds::dds::data_types::TopicKind;
  /// #
  /// # let domain_participant = DomainParticipant::new(0);
  /// # let qos = QosPolicyBuilder::new().build();
  /// #
  ///
  /// let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  ///
  /// #[derive(Deserialize)]
  /// struct SomeType {}
  ///
  /// let topic = domain_participant.create_topic("some_topic", "SomeType", &qos, TopicKind::NoKey).unwrap();
  /// let data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(topic, None, None);
  /// ```
  pub fn create_datareader_no_key<D: 'static, SA>(&self,
    topic: Topic,
    entity_id: Option<EntityId>,
    qos: Option<QosPolicies>,
  ) -> Result<NoKeyDataReader<D, SA>>
  where
    D: DeserializeOwned,
    SA: DeserializerAdapter<D>,
  {
    self.inner
      .create_datareader_no_key(self,topic,entity_id,qos)
  }

  pub fn create_datareader_no_key_CDR<D: 'static>(&self,
    topic: Topic,
    entity_id: Option<EntityId>,
    qos: Option<QosPolicies>,
  ) -> Result<NoKeyDataReader<D, CDRDeserializerAdapter<D>>>
  where
    D: DeserializeOwned,
  {
    self.create_datareader_no_key::<D,CDRDeserializerAdapter<D>>(topic,entity_id,qos)
  }


  // Retrieves a previously created DataReader belonging to the Subscriber.
  // TODO: Is this even possible. Whould probably need to return reference and store references on creation
  /*
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
    // types D and SA. Should we just trust whoever creates DataReaders to also remember them?
  }
  */

  /// Returns [DomainParticipant](struct.DomainParticipant.html) if it is sill alive.
  ///
  /// # Example
  ///
  /// ```
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::Subscriber;
  /// #
  /// let domain_participant = DomainParticipant::new(0);
  /// let qos = QosPolicyBuilder::new().build();
  ///
  /// let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  /// assert_eq!(domain_participant, subscriber.get_participant().unwrap());
  /// ```
  pub fn get_participant(&self) -> Option<DomainParticipant> {
    self.inner.get_participant()
  }
}



#[derive(Clone)]
pub struct InnerSubscriber {
  domain_participant: DomainParticipantWeak,
  discovery_db: Arc<RwLock<DiscoveryDB>>,
  qos: QosPolicies,
  sender_add_reader: mio_channel::SyncSender<Reader>,
  sender_remove_reader: mio_channel::SyncSender<GUID>,
  discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
}

impl InnerSubscriber {
  pub(super) fn new(
    domain_participant: DomainParticipantWeak,
    discovery_db: Arc<RwLock<DiscoveryDB>>,
    qos: QosPolicies,
    sender_add_reader: mio_channel::SyncSender<Reader>,
    sender_remove_reader: mio_channel::SyncSender<GUID>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  ) -> InnerSubscriber {
    InnerSubscriber {
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
    &self,
    outer: &Subscriber,
    entity_id_opt: Option<EntityId>,
    topic: Topic,
    optional_qos: Option<QosPolicies>,
  ) -> Result<WithKeyDataReader<D, SA>>
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


    let qos = optional_qos.unwrap_or_else(|| topic.get_qos().clone());

    let entity_id = unwrap_or_random_EntityId(entity_id_opt, EntityKind::READER_WITH_KEY_USER_DEFINED);

    let reader_id = entity_id;
    let datareader_id = entity_id;

    let dp = match self.get_participant() {
        Some(dp) => dp,
        None => return 
          log_and_err_precondition_not_met!("DomainParticipant doesn't exist anymore.") ,
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
      outer.clone(),
      datareader_id,
      topic.clone(),
      qos,
      rec,
      dp.get_dds_cache(),
      self.discovery_command.clone(),
      status_receiver,
      reader_command_sender,
    )?;

    {
      let mut db = self.discovery_db.write()
                .or_else(|e| log_and_err_internal!("Cannot lock discovery_db. {}",e))?;
      db.update_local_topic_reader(&dp, &topic, &new_reader);
      db.update_topic_data_p(&topic);
    }

    // Create new topic to DDScache if one isn't present
    match dp.get_dds_cache().write() {
      Ok(mut dds_cache) => {
        dds_cache.add_new_topic(
            &topic.get_name().to_string(),
            topic.kind(),
            topic.get_type(),
          );                   
      }
      Err(e) => return log_and_err_internal!("Cannot lock DDScache. Error: {}",e),
    }

    // Return the DataReader Reader pairs to where they are used
    self.sender_add_reader.try_send(new_reader)
        .or_else( |e| log_and_err_internal!("Cannot add DataReader. Error: {}",e))?;

    Ok(matching_datareader)
  }

  pub fn create_datareader<D: 'static, SA>(
    &self,
    outer: &Subscriber,
    topic: Topic,
    entity_id: Option<EntityId>,
    qos: Option<QosPolicies>,
  ) -> Result<WithKeyDataReader<D, SA>>
  where
    D: DeserializeOwned + Keyed,
    <D as Keyed>::K: Key,
    SA: DeserializerAdapter<D>,
  {
    if topic.kind() != TopicKind::WithKey {
      return Error::precondition_not_met("Topic is NO_KEY, but attempted to create WITH_KEY Datareader") 
    }
    self.create_datareader_internal(outer, entity_id, topic, qos)
  }

  pub fn create_datareader_no_key<D: 'static, SA>(
    &self,
    outer: &Subscriber,
    topic: Topic,
    entity_id_opt: Option<EntityId>,
    qos: Option<QosPolicies>,
  ) -> Result<NoKeyDataReader<D, SA>>
  where
    D: DeserializeOwned,
    SA: DeserializerAdapter<D>,
  {
    if topic.kind() != TopicKind::NoKey {
      return Error::precondition_not_met("Topic is WITH_KEY, but attempted to create NO_KEY Datareader")
    }

    let entity_id = unwrap_or_random_EntityId(entity_id_opt, EntityKind::READER_NO_KEY_USER_DEFINED);

    let d = self.create_datareader_internal::<NoKeyWrapper<D>, SAWrapper<SA>>(
      outer,
      Some(entity_id),
      topic,
      qos,
    )?;

    Ok(NoKeyDataReader::<D, SA>::from_keyed(d))
  }

  pub fn get_participant(&self) -> Option<DomainParticipant> {
    self.domain_participant.clone().upgrade()
  }
}

fn unwrap_or_random_EntityId(entity_id_opt: Option<EntityId>, entityKind: EntityKind) -> EntityId {
    entity_id_opt // use the given EntityId or generate new random one
      .unwrap_or_else( || {
            let mut rng = rand::thread_rng();
            EntityId::createCustomEntityID([rng.gen(), rng.gen(), rng.gen()], entityKind)
          } )
}

// -------------------------------------------------------------------

#[cfg(test)]
mod tests {}
