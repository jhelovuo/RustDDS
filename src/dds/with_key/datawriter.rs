use std::{
  marker::PhantomData,
  sync::{Arc, RwLock},
  time::Duration,
};

use mio::{Evented, Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel::{self as mio_channel, Receiver, SendError};
use serde::Serialize;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::{
  dds::{
    ddsdata::DDSData,
    helpers::*,
    pubsub::Publisher,
    qos::{
      policy::{Liveliness, Reliability},
      HasQoSPolicy, QosPolicies,
    },
    statusevents::*,
    topic::Topic,
    traits::{
      dds_entity::DDSEntity, key::*, serde_adapters::with_key::SerializerAdapter, TopicDescription,
    },
    values::result::{Error, Result},
  },
  discovery::{data_types::topic_data::SubscriptionBuiltinTopicData, discovery::DiscoveryCommand},
  log_and_err_internal,
  messages::submessages::submessage_elements::serialized_payload::SerializedPayload,
  serialization::CDRSerializerAdapter,
  structure::{
    cache_change::ChangeKind, dds_cache::DDSCache, entity::RTPSEntity, guid::GUID,
    rpc::SampleIdentity, time::Timestamp, topic_kind::TopicKind,
  },
};
use super::super::writer::WriterCommand;

// It is a bit overkill to use a builder for such a simple struct, but
// it may be expanded in future versions of RustDDS or even the spec.
// TODO: Move the write options and the builder type to some lower-level module
// to avoid circular dependencies.
#[derive(Debug, Default)]
pub struct WriteOptionsBuilder {
  related_sample_identity: Option<SampleIdentity>,
  source_timestamp: Option<Timestamp>,
}

impl WriteOptionsBuilder {
  pub fn new() -> WriteOptionsBuilder {
    WriteOptionsBuilder::default()
  }

  pub fn build(self) -> WriteOptions {
    WriteOptions {
      related_sample_identity: self.related_sample_identity,
      source_timestamp: self.source_timestamp,
    }
  }

  pub fn related_sample_identity(
    mut self,
    related_sample_identity: SampleIdentity,
  ) -> WriteOptionsBuilder {
    self.related_sample_identity = Some(related_sample_identity);
    self
  }

  pub fn source_timestamp(mut self, source_timestamp: Timestamp) -> WriteOptionsBuilder {
    self.source_timestamp = Some(source_timestamp);
    self
  }
}

/// Type to be used with write_with_options.
/// Use WriteOptionsBuilder to construct this.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
pub struct WriteOptions {
  pub(crate) related_sample_identity: Option<SampleIdentity>,
  pub(crate) source_timestamp: Option<Timestamp>,
  // future extension room fo other fields.
}

impl From<Option<Timestamp>> for WriteOptions {
  fn from(source_timestamp: Option<Timestamp>) -> WriteOptions {
    WriteOptions {
      related_sample_identity: None,
      source_timestamp,
    }
  }
}

/// Simplified type for CDR encoding
pub type DataWriterCdr<D> = DataWriter<D, CDRSerializerAdapter<D>>;

/// DDS DataWriter for keyed topics
///
/// # Examples
///
/// ```
/// use serde::{Serialize, Deserialize};
/// use rustdds::dds::DomainParticipant;
/// use rustdds::dds::qos::QosPolicyBuilder;
/// use rustdds::dds::data_types::TopicKind;
/// use rustdds::with_key::DataWriter;
/// use rustdds::dds::traits::Keyed;
/// use rustdds::serialization::CDRSerializerAdapter;
///
/// let domain_participant = DomainParticipant::new(0).unwrap();
/// let qos = QosPolicyBuilder::new().build();
/// let publisher = domain_participant.create_publisher(&qos).unwrap();
///
/// #[derive(Serialize, Deserialize)]
/// struct SomeType { a: i32 }
/// impl Keyed for SomeType {
///   type K = i32;
///
///   fn key(&self) -> Self::K {
///     self.a
///   }
/// }
///
/// // WithKey is important
/// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
/// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(&topic, None);
/// ```
pub struct DataWriter<D: Keyed + Serialize, SA: SerializerAdapter<D> = CDRSerializerAdapter<D>> {
  data_phantom: PhantomData<D>,
  ser_phantom: PhantomData<SA>,
  my_publisher: Publisher,
  my_topic: Topic,
  qos_policy: QosPolicies,
  my_guid: GUID,
  cc_upload: mio_channel::SyncSender<WriterCommand>,
  discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
  status_receiver: StatusReceiver<DataWriterStatus>,
}

impl<D, SA> Drop for DataWriter<D, SA>
where
  D: Keyed + Serialize,
  SA: SerializerAdapter<D>,
{
  fn drop(&mut self) {
    // Tell Publisher to drop the corresponding RTPS Writer
    self.my_publisher.remove_writer(self.my_guid);

    // Notify Discovery that we are no longer
    match self
      .discovery_command
      .send(DiscoveryCommand::RemoveLocalWriter { guid: self.guid() })
    {
      Ok(_) => {}

      // This is fairly normal at shutdown, as the other end is down already.
      Err(SendError::Disconnected(_cmd)) => {
        debug!("Failed to send REMOVE_LOCAL_WRITER DiscoveryCommand: Disconnected.");
      }
      // other errors must be taken more seriously
      Err(e) => error!(
        "Failed to send REMOVE_LOCAL_WRITER DiscoveryCommand. {:?}",
        e
      ),
    }
  }
}

impl<D, SA> DataWriter<D, SA>
where
  D: Keyed + Serialize,
  <D as Keyed>::K: Key,
  SA: SerializerAdapter<D>,
{
  pub(crate) fn new(
    publisher: Publisher,
    topic: Topic,
    guid: GUID,
    cc_upload: mio_channel::SyncSender<WriterCommand>,
    discovery_command: mio_channel::SyncSender<DiscoveryCommand>,
    dds_cache: &Arc<RwLock<DDSCache>>, // Apparently, this is only needed for our Topic creation
    status_receiver_rec: Receiver<DataWriterStatus>,
  ) -> Result<DataWriter<D, SA>> {
    match dds_cache.write() {
      Ok(mut cache) => cache.add_new_topic(topic.name(), TopicKind::NoKey, topic.get_type()),
      Err(e) => panic!("DDSCache is poisoned. {:?}", e),
    };

    if let Some(lv) = topic.qos().liveliness {
      match lv {
        Liveliness::Automatic { .. } | Liveliness::ManualByTopic { .. } => (),
        Liveliness::ManualByParticipant { .. } => {
          if let Err(e) = discovery_command.send(DiscoveryCommand::ManualAssertLiveliness) {
            error!("Failed to send DiscoveryCommand - Refresh. {:?}", e);
          }
        }
      }
    };
    let qos = topic.qos();
    Ok(DataWriter {
      data_phantom: PhantomData,
      ser_phantom: PhantomData,
      my_publisher: publisher,
      my_topic: topic,
      qos_policy: qos,
      my_guid: guid,
      cc_upload,
      discovery_command,
      status_receiver: StatusReceiver::new(status_receiver_rec),
    })
  }

  // This one function provides both get_matched_subscrptions and
  // get_matched_subscription_data TODO: Maybe we could return references to the
  // subscription data to avoid copying? But then what if the result set changes
  // while the application processes it?

  /// Manually refreshes liveliness if QoS allows it
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(&topic, None).unwrap();
  ///
  /// data_writer.refresh_manual_liveliness();
  /// ```

  // TODO: What is this function? To what part of DDS spec does it correspond to?
  pub fn refresh_manual_liveliness(&self) {
    if let Some(lv) = self.qos().liveliness {
      match lv {
        Liveliness::Automatic { .. } | Liveliness::ManualByTopic { .. } => (),
        Liveliness::ManualByParticipant { .. } => {
          if let Err(e) = self
            .discovery_command
            .send(DiscoveryCommand::ManualAssertLiveliness)
          {
            error!("Failed to send DiscoveryCommand - Refresh. {:?}", e);
          }
        }
      }
    };
  }

  /// Writes single data instance to a topic.
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(&topic, None).unwrap();
  ///
  /// let some_data = SomeType { a: 1 };
  /// data_writer.write(some_data, None).unwrap();
  /// ```
  pub fn write(&self, data: D, source_timestamp: Option<Timestamp>) -> Result<()> {
    self.write_with_options(data, WriteOptions::from(source_timestamp))
  }

  pub fn write_with_options(&self, data: D, write_options: WriteOptions) -> Result<()> {
    let send_buffer = SA::to_bytes(&data)?; // serialize

    let ddsdata = DDSData::new(SerializedPayload::new_from_bytes(
      SA::output_encoding(),
      send_buffer,
    ));
    let writer_command = WriterCommand::DDSData {
      data: ddsdata,
      write_options,
    };

    let timeout = match self.qos().reliability() {
      Some(Reliability::Reliable { max_blocking_time }) => Some(max_blocking_time),
      _ => None,
    };

    match try_send_timeout(&self.cc_upload, writer_command, timeout) {
      Ok(_) => {
        self.refresh_manual_liveliness();
        Ok(())
      }
      Err(e) => {
        warn!(
          "Failed to write new data: topic={:?}  reason={:?}  timeout={:?}",
          self.my_topic.name(),
          e,
          timeout,
        );
        Err(Error::OutOfResources)
      }
    }
  }

  /// This operation blocks the calling thread until either all data written by
  /// the reliable DataWriter entities is acknowledged by all
  /// matched reliable DataReader entities, or else the duration specified by
  /// the `max_wait` parameter elapses, whichever happens first.
  ///
  /// See DDS Spec 1.4 Section 2.2.2.4.1.12 wait_for_acknowledgments.
  ///
  /// If this DataWriter is not set to Realiable, or there are no matched
  /// DataReaders with Realibale QoS, the call succeeds imediately.
  ///
  /// Return values
  /// * `Ok(true)` - all acknowledged
  /// * `Ok(false)`- timed out waiting for acknowledgments
  /// * `Err(_)` - something went wrong
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(&topic, None).unwrap();
  ///
  /// let some_data = SomeType { a: 1 };
  /// data_writer.write(some_data, None).unwrap();
  /// data_writer.wait_for_acknowledgments(std::time::Duration::from_millis(100));
  /// ```
  pub fn wait_for_acknowledgments(&self, max_wait: Duration) -> Result<bool> {
    match &self.qos_policy.reliability {
      None | Some(Reliability::BestEffort) => Ok(true),
      Some(Reliability::Reliable { .. }) => {
        let (acked_sender, acked_receiver) = mio_channel::sync_channel::<()>(1);
        let poll = Poll::new()?;
        poll.register(
          &acked_receiver,
          Token(0),
          Ready::readable(),
          PollOpt::edge(),
        )?;
        self
          .cc_upload
          .try_send(WriterCommand::WaitForAcknowledgments {
            all_acked: acked_sender,
          })?;
        let mut events = Events::with_capacity(1);
        poll.poll(&mut events, Some(max_wait))?;
        if let Some(_event) = events.iter().next() {
          let _ = acked_receiver
            .try_recv()
            .or_else(|_e| log_and_err_internal!("wait_for_acknowledgments - Spurious poll event?"));
          // got reply
          Ok(true)
        } else {
          // no token, so presumably timed out
          Ok(false)
        }
      }
    } // match
  }
  /*
  /// Gets mio Receiver for all status changes
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// // Some status has changed
  ///
  /// while let Ok(sc) = data_writer.get_status_listener().try_recv() {
  ///   // do something
  /// }
  /// ```
  pub fn get_status_listener(&self) -> &Receiver<StatusChange> {
    match self
      .cc_upload
      .try_send(WriterCommand::ResetOfferedDeadlineMissedStatus {
        writer_guid: self.guid(),
      }) {
      Ok(_) => (),
      Err(e) => error!("Unable to send ResetOfferedDeadlineMissedStatus. {:?}", e),
    };
    &self.status_receiver
  }

  /// Unimplemented. <b>Do not use</b>.
  ///
  /// # Examples
  ///
  /// ```no_run
  // TODO: enable when functional
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// // Liveliness lost status has changed
  ///
  /// if let Ok(lls) = data_writer.get_liveliness_lost_status() {
  ///   // do something
  /// }
  /// ```
  pub fn get_liveliness_lost_status(&self) -> Result<LivelinessLostStatus> {
    todo!()
  }

  /// Should get latest offered deadline missed status. <b>Do not use yet</b> use `get_status_lister` instead for the moment.
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// // Deadline missed status has changed
  ///
  /// if let Ok(odms) = data_writer.get_offered_deadline_missed_status() {
  ///   // do something
  /// }
  /// ```
  pub fn get_offered_deadline_missed_status(&self) -> Result<OfferedDeadlineMissedStatus> {
    let mut fstatus = OfferedDeadlineMissedStatus::new();
    while let Ok(status) = self.status_receiver.try_recv() {
      match status {
        StatusChange::OfferedDeadlineMissedStatus(status) => fstatus = status,
        // TODO: possibly save old statuses
        _ => (),
      }
    }

    match self
      .cc_upload
      .try_send(WriterCommand::ResetOfferedDeadlineMissedStatus {
        writer_guid: self.guid(),
      }) {
      Ok(_) => (),
      Err(e) => error!("Unable to send ResetOfferedDeadlineMissedStatus. {:?}", e),
    };

    Ok(fstatus)
  }

  /// Unimplemented. <b>Do not use</b>.
  ///
  /// # Examples
  ///
  /// ```no_run
  // TODO: enable when functional
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// // Liveliness lost status has changed
  ///
  /// if let Ok(oiqs) = data_writer.get_offered_incompatible_qos_status() {
  ///   // do something
  /// }
  /// ```
  pub fn get_offered_incompatible_qos_status(&self) -> Result<OfferedIncompatibleQosStatus> {
    todo!()
  }

  /// Unimplemented. <b>Do not use</b>.
  ///
  /// # Examples
  ///
  /// ```no_run
  // TODO: enable when functional
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(topic, None).unwrap();
  ///
  /// // Liveliness lost status has changed
  ///
  /// if let Ok(pms) = data_writer.get_publication_matched_status() {
  ///   // do something
  /// }
  /// ```
  pub fn get_publication_matched_status(&self) -> Result<PublicationMatchedStatus> {
    todo!()
  }

  */

  /// Topic assigned to this DataWriter
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(&topic, None).unwrap();
  ///
  /// assert_eq!(data_writer.topic(), &topic);
  /// ```
  pub fn topic(&self) -> &Topic {
    &self.my_topic
  }

  /// Publisher assigned to this DataWriter
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(&topic, None).unwrap();
  ///
  /// assert_eq!(data_writer.publisher(), &publisher);
  pub fn publisher(&self) -> &Publisher {
    &self.my_publisher
  }

  /// Manually asserts liveliness (use this instead of refresh) according to QoS
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(&topic, None).unwrap();
  ///
  /// data_writer.assert_liveliness().unwrap();
  /// ```

  // TODO: This cannot really fail, so could change type to () (alternatively,
  // make send error visible) TODO: Better make send failure visible, so
  // application can see if Discovery has failed.
  pub fn assert_liveliness(&self) -> Result<()> {
    self.refresh_manual_liveliness();

    match self.qos().liveliness {
      Some(Liveliness::ManualByTopic { lease_duration: _ }) => {
        self
          .discovery_command
          .send(DiscoveryCommand::AssertTopicLiveliness {
            writer_guid: self.guid(),
            manual_assertion: true, // by definition of this function
          })
          .unwrap_or_else(|e| error!("assert_liveness - Failed to send DiscoveryCommand. {:?}", e));
      }
      _other => (),
    }
    Ok(())
  }

  /// Unimplemented. <b>Do not use</b>.
  ///
  /// # Examples
  ///
  /// ```no_run
  // TODO: enable when available
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32 }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(),
  /// "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType,
  /// CDRSerializerAdapter<_>>(&topic, None).unwrap();
  ///
  /// for sub in data_writer.get_matched_subscriptions().iter() {
  ///   // do something
  /// }
  pub fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData> {
    todo!()
  }

  /// Disposes data instance with specified key
  ///
  /// # Arguments
  ///
  /// * `key` - Key of the instance
  /// * `source_timestamp` - DDS source timestamp (None uses now as time as
  ///   specified in DDS spec)
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::with_key::DataWriter;
  /// # use rustdds::dds::traits::Keyed;
  /// # use rustdds::serialization::CDRSerializerAdapter;
  /// #
  /// let domain_participant = DomainParticipant::new(0).unwrap();
  /// let qos = QosPolicyBuilder::new().build();
  /// let publisher = domain_participant.create_publisher(&qos).unwrap();
  ///
  /// #[derive(Serialize, Deserialize)]
  /// struct SomeType { a: i32, val: usize }
  /// impl Keyed for SomeType {
  ///   type K = i32;
  ///
  ///   fn key(&self) -> Self::K {
  ///     self.a
  ///   }
  /// }
  ///
  /// // WithKey is important
  /// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::WithKey).unwrap();
  /// let data_writer = publisher.create_datawriter::<SomeType, CDRSerializerAdapter<_>>(&topic, None).unwrap();
  ///
  /// let some_data_1_1 = SomeType { a: 1, val: 3};
  /// let some_data_1_2 = SomeType { a: 1, val: 4};
  /// // different key
  /// let some_data_2_1 = SomeType { a: 2, val: 5};
  /// let some_data_2_2 = SomeType { a: 2, val: 6};
  ///
  /// data_writer.write(some_data_1_1, None).unwrap();
  /// data_writer.write(some_data_1_2, None).unwrap();
  /// data_writer.write(some_data_2_1, None).unwrap();
  /// data_writer.write(some_data_2_2, None).unwrap();
  ///
  /// // disposes both some_data_1_1 and some_data_1_2. They are no longer offered by this writer to this topic.
  /// data_writer.dispose(&1, None).unwrap();
  /// ```
  pub fn dispose(&self, key: &<D as Keyed>::K, source_timestamp: Option<Timestamp>) -> Result<()> {
    let send_buffer = SA::key_to_bytes(key)?; // serialize key

    let ddsdata = DDSData::new_disposed_by_key(
      ChangeKind::NotAliveDisposed,
      SerializedPayload::new_from_bytes(SA::output_encoding(), send_buffer),
    );
    self
      .cc_upload
      .send(WriterCommand::DDSData {
        data: ddsdata,
        write_options: WriteOptions::from(source_timestamp),
      })
      .or_else(|huh| log_and_err_internal!("Cannot send dispose command: {:?}", huh))?;

    self.refresh_manual_liveliness();
    Ok(())
  }
}

impl<D, SA> StatusEvented<DataWriterStatus> for DataWriter<D, SA>
where
  D: Keyed + Serialize,
  SA: SerializerAdapter<D>,
{
  fn as_status_evented(&mut self) -> &dyn Evented {
    self.status_receiver.as_status_evented()
  }

  fn try_recv_status(&self) -> Option<DataWriterStatus> {
    self.status_receiver.try_recv_status()
  }
}

impl<D, SA> RTPSEntity for DataWriter<D, SA>
where
  D: Keyed + Serialize,
  SA: SerializerAdapter<D>,
{
  fn guid(&self) -> GUID {
    self.my_guid
  }
}

impl<D, SA> HasQoSPolicy for DataWriter<D, SA>
where
  D: Keyed + Serialize,
  SA: SerializerAdapter<D>,
{
  // fn set_qos(&mut self, policy: &QosPolicies) -> Result<()> {
  //   // TODO: check liveliness of qos_policy
  //   self.qos_policy = policy.clone();
  //   Ok(())
  // }

  fn qos(&self) -> QosPolicies {
    self.qos_policy.clone()
  }
}

impl<D, SA> DDSEntity for DataWriter<D, SA>
where
  D: Keyed + Serialize,
  SA: SerializerAdapter<D>,
{
}

#[cfg(test)]
mod tests {
  use std::thread;

  use byteorder::LittleEndian;
  use log::info;

  use super::*;
  use crate::{
    dds::{participant::DomainParticipant, traits::key::Keyed},
    serialization::cdr_serializer::CDRSerializerAdapter,
    test::random_data::*,
  };

  #[test]
  fn dw_write_test() {
    let domain_participant = DomainParticipant::new(0).expect("Publisher creation failed!");
    let qos = QosPolicies::qos_none();
    let _default_dw_qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic(
        "Aasii".to_string(),
        "Huh?".to_string(),
        &qos,
        TopicKind::WithKey,
      )
      .expect("Failed to create topic");

    let data_writer: DataWriter<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter(&topic, None)
        .expect("Failed to create datawriter");

    let mut data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    data_writer
      .write(data.clone(), None)
      .expect("Unable to write data");

    data.a = 5;
    let timestamp = Timestamp::now();
    data_writer
      .write(data, Some(timestamp))
      .expect("Unable to write data with timestamp");

    // TODO: verify that data is sent/writtent correctly
    // TODO: write also with timestamp
  }

  #[test]
  fn dw_dispose_test() {
    let domain_participant = DomainParticipant::new(0).expect("Publisher creation failed!");
    let qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic(
        "Aasii".to_string(),
        "Huh?".to_string(),
        &qos,
        TopicKind::WithKey,
      )
      .expect("Failed to create topic");

    let data_writer: DataWriter<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter(&topic, None)
        .expect("Failed to create datawriter");

    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    let key = &data.key().hash_key();
    info!("key: {:?}", key);

    data_writer
      .write(data.clone(), None)
      .expect("Unable to write data");

    thread::sleep(Duration::from_millis(100));
    data_writer
      .dispose(&data.key(), None)
      .expect("Unable to dispose data");

    // TODO: verify that dispose is sent correctly
  }

  #[test]
  fn dw_wait_for_ack_test() {
    let domain_participant = DomainParticipant::new(0).expect("Participant creation failed!");
    let qos = QosPolicies::qos_none();
    let publisher = domain_participant
      .create_publisher(&qos)
      .expect("Failed to create publisher");
    let topic = domain_participant
      .create_topic(
        "Aasii".to_string(),
        "Huh?".to_string(),
        &qos,
        TopicKind::WithKey,
      )
      .expect("Failed to create topic");

    let data_writer: DataWriter<RandomData, CDRSerializerAdapter<RandomData, LittleEndian>> =
      publisher
        .create_datawriter(&topic, None)
        .expect("Failed to create datawriter");

    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    data_writer.write(data, None).expect("Unable to write data");

    let res = data_writer
      .wait_for_acknowledgments(Duration::from_secs(2))
      .unwrap();
    assert!(res); // we should get "true" immediately, because we have
                  // no Reliable QoS
  }
}
