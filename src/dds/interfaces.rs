use std::time::Duration;

use mio::Evented;
use mio_extras::channel::Receiver;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
  discovery::data_types::topic_data::SubscriptionBuiltinTopicData,
  structure::{entity::Entity, time::Timestamp},
};

use super::{
  qos::HasQoSPolicy, datareader::SelectByKey, datasample::SampleInfo, pubsub::Publisher,
  readcondition::ReadCondition, topic::Topic, traits::key::Key, traits::key::Keyed,
  traits::serde_adapters::DeserializerAdapter, traits::serde_adapters::SerializerAdapter,
  values::result::Error, values::result::LivelinessLostStatus,
  values::result::OfferedDeadlineMissedStatus, values::result::OfferedIncompatibleQosStatus,
  values::result::PublicationMatchedStatus, values::result::RequestedDeadlineMissedStatus,
  values::result::StatusChange,
};

type Result<T> = std::result::Result<T, Error>;

pub trait IDataWriter<D, SA>: Entity + HasQoSPolicy
where
  D: Serialize,
  SA: SerializerAdapter<D>,
{
  // write (with optional timestamp)
  // This operation could take also in InstanceHandle, if we would use them.
  // The _with_timestamp version is covered by the optional timestamp.
  fn write(&self, data: D, source_timestamp: Option<Timestamp>) -> Result<()>;
  fn wait_for_acknowledgments(&self, _max_wait: Duration) -> Result<()>;

  // status
  fn get_status_listener(&self) -> &Receiver<StatusChange>;
  fn get_liveliness_lost_status(&self) -> Result<LivelinessLostStatus>;
  fn get_offered_deadline_missed_status(&self) -> Result<OfferedDeadlineMissedStatus>;
  fn get_offered_incompatible_qos_status(&self) -> Result<OfferedIncompatibleQosStatus>;
  fn get_publication_matched_status(&self) -> Result<PublicationMatchedStatus>;

  // general
  fn get_topic(&self) -> &Topic;
  fn get_publisher(&self) -> &Publisher;
  fn assert_liveliness(&self) -> Result<()>;

  // DDS spec returns an InstanceHandles pointing to a BuiltInTopic reader
  //  but we do not use those, so return the actual data instead.
  fn get_matched_subscriptions(&self) -> Vec<SubscriptionBuiltinTopicData>;
}

pub trait IKeyedDataWriter<D, SA>: IDataWriter<D, SA>
where
  D: Keyed + Serialize,
  <D as Keyed>::K: Key,
  SA: SerializerAdapter<D>,
{
  // dispose
  // The data item is given only for identification, i.e. extracting the key
  fn dispose(&mut self, key: <D as Keyed>::K, source_timestamp: Option<Timestamp>) -> Result<()>;
}

pub trait IDataSample<D>
where
  D: Sized,
{
  fn get_sample_info(&self) -> &SampleInfo;
  fn get_sample_info_mut(&mut self) -> &mut SampleInfo;

  // either there is a value or there is not. Use IKeyedDataSample for keyed
  fn get_value(&self) -> Option<&D>;
  fn into_value(self) -> Option<D>;

  // conversions
  fn as_idata_sample(&self) -> &dyn IDataSample<D>;
  fn into_idata_sample(self) -> Box<dyn IDataSample<D>>;
}

pub trait IKeyedDataSample<D>: IDataSample<D>
where
  D: Keyed,
{
  fn get_keyed_value(&self) -> &std::result::Result<D, D::K>;

  // conversions
  fn as_ikeyed_data_sample(&self) -> &dyn IKeyedDataSample<D>;
  fn into_ikeyed_data_sample(self) -> Box<dyn IKeyedDataSample<D>>;
}

pub trait IDataReader<D, DA>: Evented + Entity + HasQoSPolicy
where
  D: DeserializeOwned,
  DA: DeserializerAdapter<D>,
{
  /// This operation accesses a collection of Data values from the DataReader.
  /// The function return references to the data.
  /// The size of the returned collection will be limited to the specified max_samples.
  /// The read_condition filters the samples accessed.
  /// If no matching samples exist, then an empty Vec will be returned, but that is not an error.
  /// In case a disposed sample is returned, it is indicated by InstanceState in SampleInfo and
  /// DataSample.value will be Err containig the key of the disposed sample.
  ///
  /// View state and sample state are maintained for each DataReader separately. Instance state is
  /// determined by global CacheChange objects concerning that instance.
  ///
  /// If the sample belongs to the most recent generation of the instance,
  /// it will also set the view_state of the instance to NotNew.
  ///
  /// This should cover DDS DataReader methods read, read_w_condition and
  /// read_next_sample.
  fn read(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<&dyn IDataSample<D>>>;

  /// Similar to read, but insted of references being returned, the datasamples
  /// are removed from the DataReader and ownership is transferred to the caller.
  /// Should cover take, take_w_condition and take_next_sample
  fn take(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<Box<dyn IDataSample<D>>>>;

  /// This is a simplified API for reading the next not_read sample
  /// If no new data is available, the return value is Ok(None).
  fn read_next_sample(&mut self) -> Result<Option<&dyn IDataSample<D>>>;
  fn take_next_sample(&mut self) -> Result<Option<Box<dyn IDataSample<D>>>>;

  // status queries
  fn get_requested_deadline_missed_status(&self) -> Result<RequestedDeadlineMissedStatus>;
}

pub trait IKeyedDataReader<D, DA>: IDataReader<D, DA>
where
  D: DeserializeOwned + Keyed,
  <D as Keyed>::K: Key,
  DA: DeserializerAdapter<D>,
{
  fn read(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<&dyn IKeyedDataSample<D>>>;

  fn take(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
  ) -> Result<Vec<Box<dyn IKeyedDataSample<D>>>>;

  fn read_next_sample(&mut self) -> Result<Option<&dyn IKeyedDataSample<D>>>;
  fn take_next_sample(&mut self) -> Result<Option<Box<dyn IKeyedDataSample<D>>>>;

  fn read_instance(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
    // Select only samples from instance specified by key. In case of None, select the
    // "smallest" instance as specified by the key type Ord trait.
    instance_key: Option<<D as Keyed>::K>,
    // This = Select instance specified by key.
    // Next = select next instance in the order specified by Ord on keys.
    this_or_next: SelectByKey,
  ) -> Result<Vec<&dyn IKeyedDataSample<D>>>;

  fn take_instance(
    &mut self,
    max_samples: usize,
    read_condition: ReadCondition,
    // Select only samples from instance specified by key. In case of None, select the
    // "smallest" instance as specified by the key type Ord trait.
    instance_key: Option<<D as Keyed>::K>,
    // This = Select instance specified by key.
    // Next = select next instance in the order specified by Ord on keys.
    this_or_next: SelectByKey,
  ) -> Result<Vec<Box<dyn IKeyedDataSample<D>>>>;
}
