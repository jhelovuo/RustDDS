use std::time::Duration;

use serde::{Serialize, Deserialize};

use crate::structure::result::*;
use crate::structure::time::Timestamp;

use crate::dds::participant::*;
use crate::dds::topic::*;
use crate::dds::key::*;

// -------------------------------------------------------------------

pub struct Publisher<'a> {
  my_domainparticipant: &'a DomainParticipant,
  my_qos_policies: QosPolicies,
  default_datawriter_qos: QosPolicies, // used when creating a new DataWriter
}

pub struct Subscriber<'a> {
  my_domainparticipant: &'a DomainParticipant,
}

// public interface for Publisher
impl<'a> Publisher<'a> {
  pub fn create_datawriter<'p, D>(
    &self,
    a_topic: &Topic,
    qos: QosPolicies,
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
  pub fn wait_for_acknowledgments(&self, max_wait: Duration) -> Result<()> {
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
}

impl<'a> Subscriber<'a> {
  pub fn create_datareader<'p, D>(
    &self,
    a_topic: &Topic,
    qos: QosPolicies,
  ) -> Result<DataReader<'p>> {
    unimplemented!();
  }
}

// -------------------------------------------------------------------

pub struct DataReader<'s> {
  my_subscriber: &'s Subscriber<'s>,
  // TODO: rest of fields
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
  pub fn write<D>(&self, data: D, source_timestamp: Option<Timestamp>)
  where
    D: Serialize + Keyed,
  {
  }

  // dispose
  // The data item is given only for identification, i.e. extracting the key
  pub fn dispose<D>(&self, data: &D, source_timestamp: Option<Timestamp>)
  where
    D: Serialize + Keyed,
  {
  }

  pub fn wait_for_acknowledgments(&self, max_wait: Duration) -> Result<()> {
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
