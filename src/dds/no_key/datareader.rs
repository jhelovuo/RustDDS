use std::io;

use serde::de::DeserializeOwned;
use mio_06::{Evented, Poll, PollOpt, Ready, Token};

use crate::{
  dds::{
    data_types::GUID,
    no_key::datasample::DataSample,
    qos::{HasQoSPolicy, QosPolicies},
    readcondition::ReadCondition,
    traits::serde_adapters::no_key::DeserializerAdapter,
    values::result::Result,
    with_key::{datareader as datareader_with_key, datasample::DataSample as WithKeyDataSample},
  },
  serialization::CDRDeserializerAdapter,
  structure::entity::RTPSEntity,
};
use super::wrappers::{DAWrapper, NoKeyWrapper};

/// Simplified type for CDR encoding
pub type DataReaderCdr<D> = DataReader<D, CDRDeserializerAdapter<D>>;

// ----------------------------------------------------

// DataReader for NO_KEY data. Does not require "D: Keyed"
/// DDS DataReader for no key topics.
/// # Examples
///
/// ```
/// use serde::{Serialize, Deserialize};
/// use rustdds::dds::DomainParticipant;
/// use rustdds::dds::qos::QosPolicyBuilder;
/// use rustdds::dds::data_types::TopicKind;
/// use rustdds::no_key::DataReader;
/// use rustdds::serialization::CDRDeserializerAdapter;
///
/// let domain_participant = DomainParticipant::new(0).unwrap();
/// let qos = QosPolicyBuilder::new().build();
/// let subscriber = domain_participant.create_subscriber(&qos).unwrap();
///
/// #[derive(Serialize, Deserialize)]
/// struct SomeType {}
///
/// // NoKey is important
/// let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
/// let data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(&topic, None);
/// ```
pub struct DataReader<D: DeserializeOwned, DA: DeserializerAdapter<D> = CDRDeserializerAdapter<D>> {
  keyed_datareader: datareader_with_key::DataReader<NoKeyWrapper<D>, DAWrapper<DA>>,
}

// TODO: rewrite DataSample so it can use current Keyed version (and send back
// datasamples instead of current data)
impl<D: 'static, DA> DataReader<D, DA>
where
  D: DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  pub(crate) fn from_keyed(
    keyed: datareader_with_key::DataReader<NoKeyWrapper<D>, DAWrapper<DA>>,
  ) -> Self {
    Self {
      keyed_datareader: keyed,
    }
  }

  /// Reads amount of samples found with `max_samples` and `read_condition`
  /// parameters.
  ///
  /// # Arguments
  ///
  /// * `max_samples` - Limits maximum amount of samples read
  /// * `read_condition` - Limits results by condition
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::no_key::DataReader;
  /// # use rustdds::serialization::CDRDeserializerAdapter;
  /// use rustdds::dds::data_types::ReadCondition;
  ///
  /// # let domain_participant = DomainParticipant::new(0).unwrap();
  /// # let qos = QosPolicyBuilder::new().build();
  /// # let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  /// #
  /// # // NoKey is important
  /// # let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// #
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// let mut data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(&topic, None).unwrap();
  /// let data = data_reader.read(10, ReadCondition::not_read());
  /// ```
  pub fn read(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<DataSample<&D>>> {
    let values: Vec<WithKeyDataSample<&NoKeyWrapper<D>>> =
      self.keyed_datareader.read(max_samples, read_condition)?;
    let mut result = Vec::with_capacity(values.len());
    for ks in values {
      if let Some(s) = DataSample::<D>::from_with_key_ref(ks) {
        result.push(s);
      }
    }
    Ok(result)
  }

  /// Takes amount of sample found with `max_samples` and `read_condition`
  /// parameters.
  ///
  /// # Arguments
  ///
  /// * `max_samples` - Limits maximum amount of samples read
  /// * `read_condition` - Limits results by condition
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::no_key::DataReader;
  /// # use rustdds::serialization::CDRDeserializerAdapter;
  /// # use rustdds::dds::data_types::ReadCondition;
  /// #
  /// # let domain_participant = DomainParticipant::new(0).unwrap();
  /// # let qos = QosPolicyBuilder::new().build();
  /// # let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  /// #
  /// # // NoKey is important
  /// # let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// #
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// let mut data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(&topic, None).unwrap();
  /// let data = data_reader.take(10, ReadCondition::not_read());
  /// ```
  pub fn take(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<DataSample<D>>> {
    let values: Vec<WithKeyDataSample<NoKeyWrapper<D>>> =
      self.keyed_datareader.take(max_samples, read_condition)?;
    let mut result = Vec::with_capacity(values.len());
    for ks in values {
      if let Some(s) = DataSample::<D>::from_with_key(ks) {
        result.push(s);
      }
    }
    Ok(result)
  }

  /// Reads next unread sample
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::no_key::DataReader;
  /// # use rustdds::serialization::CDRDeserializerAdapter;
  /// #
  /// # let domain_participant = DomainParticipant::new(0).unwrap();
  /// # let qos = QosPolicyBuilder::new().build();
  /// # let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  /// #
  /// # // NoKey is important
  /// # let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// #
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// let mut data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(&topic, None).unwrap();
  /// while let Ok(Some(data)) = data_reader.read_next_sample() {
  ///   // Do something
  /// }
  /// ```
  pub fn read_next_sample(&mut self) -> Result<Option<DataSample<&D>>> {
    let mut ds = self.read(1, ReadCondition::not_read())?;
    Ok(ds.pop())
  }

  /// Takes next unread sample
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::no_key::DataReader;
  /// # use rustdds::serialization::CDRDeserializerAdapter;
  /// #
  /// # let domain_participant = DomainParticipant::new(0).unwrap();
  /// # let qos = QosPolicyBuilder::new().build();
  /// # let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  /// #
  /// # // NoKey is important
  /// # let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// #
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// let mut data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(&topic, None).unwrap();
  /// while let Ok(Some(data)) = data_reader.take_next_sample() {
  ///   // Do something
  /// }
  /// ```
  pub fn take_next_sample(&mut self) -> Result<Option<DataSample<D>>> {
    let mut ds = self.take(1, ReadCondition::not_read())?;
    Ok(ds.pop())
  }

  // Iterator interface

  /// Produces an interator over the currently available NOT_READ samples.
  /// Yields only payload data, not SampleInfo metadata
  /// This is not called `iter()` because it takes a mutable reference to self.
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::TopicKind;
  /// # use rustdds::no_key::DataReader;
  /// # use rustdds::serialization::CDRDeserializerAdapter;
  /// #
  /// # let domain_participant = DomainParticipant::new(0).unwrap();
  /// # let qos = QosPolicyBuilder::new().build();
  /// # let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  /// #
  /// # // NoKey is important
  /// # let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// #
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// let mut data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(&topic, None).unwrap();
  /// for data in data_reader.iterator() {
  ///   // Do something
  /// }
  /// ```
  pub fn iterator(&mut self) -> Result<impl Iterator<Item = &D>> {
    // TODO: We could come up with a more efficent implementation than wrapping a
    // read call
    Ok(
      self
        .read(std::usize::MAX, ReadCondition::not_read())?
        .into_iter()
        .map(|ds| ds.value),
    )
  }

  /// Produces an interator over the samples filtered b ygiven condition.
  /// Yields only payload data, not SampleInfo metadata
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::{TopicKind, ReadCondition};
  /// # use rustdds::no_key::DataReader;
  /// # use rustdds::serialization::CDRDeserializerAdapter;
  /// #
  /// # let domain_participant = DomainParticipant::new(0).unwrap();
  /// # let qos = QosPolicyBuilder::new().build();
  /// # let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  /// #
  /// # // NoKey is important
  /// # let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// #
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// let mut data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(&topic, None).unwrap();
  /// for data in data_reader.conditional_iterator(ReadCondition::any()) {
  ///   // Do something
  /// }
  /// ```
  pub fn conditional_iterator(
    &mut self,
    read_condition: ReadCondition,
  ) -> Result<impl Iterator<Item = &D>> {
    // TODO: We could come up with a more efficent implementation than wrapping a
    // read call
    Ok(
      self
        .read(std::usize::MAX, read_condition)?
        .into_iter()
        .map(|ds| ds.value),
    )
  }

  /// Produces an interator over the currently available NOT_READ samples.
  /// Yields only payload data, not SampleInfo metadata
  /// Removes samples from `DataReader`.
  /// <strong>Note!</strong> If the iterator is only partially consumed, all the
  /// samples it could have provided are still removed from the `Datareader`.
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::{TopicKind, ReadCondition};
  /// # use rustdds::no_key::DataReader;
  /// # use rustdds::serialization::CDRDeserializerAdapter;
  /// #
  /// # let domain_participant = DomainParticipant::new(0).unwrap();
  /// # let qos = QosPolicyBuilder::new().build();
  /// # let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  /// #
  /// # // NoKey is important
  /// # let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// #
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// let mut data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(&topic, None).unwrap();
  /// for data in data_reader.into_iterator() {
  ///   // Do something
  /// }
  /// ```
  pub fn into_iterator(&mut self) -> Result<impl Iterator<Item = D>> {
    // TODO: We could come up with a more efficent implementation than wrapping a
    // read call
    Ok(
      self
        .take(std::usize::MAX, ReadCondition::not_read())?
        .into_iter()
        .map(|ds| ds.value),
    )
  }

  /// Produces an interator over the samples filtered b ygiven condition.
  /// Yields only payload data, not SampleInfo metadata
  /// <strong>Note!</strong> If the iterator is only partially consumed, all the
  /// samples it could have provided are still removed from the `Datareader`.
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::{TopicKind, ReadCondition};
  /// # use rustdds::no_key::DataReader;
  /// # use rustdds::serialization::CDRDeserializerAdapter;
  /// #
  /// # let domain_participant = DomainParticipant::new(0).unwrap();
  /// # let qos = QosPolicyBuilder::new().build();
  /// # let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  /// #
  /// # // NoKey is important
  /// # let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// #
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// let mut data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(&topic, None).unwrap();
  /// for data in data_reader.into_conditional_iterator(ReadCondition::any()) {
  ///   // Do something
  /// }
  /// ```
  pub fn into_conditional_iterator(
    &mut self,
    read_condition: ReadCondition,
  ) -> Result<impl Iterator<Item = D>> {
    // TODO: We could come up with a more efficent implementation than wrapping a
    // read call
    Ok(
      self
        .take(std::usize::MAX, read_condition)?
        .into_iter()
        .map(|ds| ds.value),
    )
  }
  /*
  /// Gets latest RequestedDeadlineMissed status
  ///
  /// # Examples
  ///
  /// ```
  /// # use serde::{Serialize, Deserialize};
  /// # use rustdds::dds::DomainParticipant;
  /// # use rustdds::dds::qos::QosPolicyBuilder;
  /// # use rustdds::dds::data_types::{TopicKind, ReadCondition};
  /// # use rustdds::no_key::DataReader;
  /// # use rustdds::serialization::CDRDeserializerAdapter;
  /// #
  /// # let domain_participant = DomainParticipant::new(0).unwrap();
  /// # let qos = QosPolicyBuilder::new().build();
  /// # let subscriber = domain_participant.create_subscriber(&qos).unwrap();
  /// #
  /// # // NoKey is important
  /// # let topic = domain_participant.create_topic("some_topic".to_string(), "SomeType".to_string(), &qos, TopicKind::NoKey).unwrap();
  /// #
  /// # #[derive(Serialize, Deserialize)]
  /// # struct SomeType {}
  /// #
  /// let mut data_reader = subscriber.create_datareader_no_key::<SomeType, CDRDeserializerAdapter<_>>(topic, None).unwrap();
  /// if let Ok(Some(status)) = data_reader.get_requested_deadline_missed_status() {
  ///   // Do something
  /// }
  /// ```
  pub fn get_requested_deadline_missed_status(
    &mut self,
  ) -> Result<Option<RequestedDeadlineMissedStatus>> {
    self.keyed_datareader.get_requested_deadline_missed_status()
  }
  */
}

// This is  not part of DDS spec. We implement mio Eventd so that the
// application can asynchronously poll DataReader(s).
impl<D, DA> Evented for DataReader<D, DA>
where
  D: DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  // We just delegate all the operations to notification_receiver, since it alrady
  // implements Evented
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self
      .keyed_datareader
      .notification_receiver
      .register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &Poll,
    token: Token,
    interest: Ready,
    opts: PollOpt,
  ) -> io::Result<()> {
    self
      .keyed_datareader
      .notification_receiver
      .reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.keyed_datareader.notification_receiver.deregister(poll)
  }
}

impl<D, DA> HasQoSPolicy for DataReader<D, DA>
where
  D: DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  // fn set_qos(&mut self, policy: &QosPolicies) -> Result<()> {
  //   self.keyed_datareader.set_qos(policy)
  // }

  fn qos(&self) -> QosPolicies {
    self.keyed_datareader.qos()
  }
}

impl<D, DA> RTPSEntity for DataReader<D, DA>
where
  D: DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  fn guid(&self) -> GUID {
    self.keyed_datareader.guid()
  }
}
