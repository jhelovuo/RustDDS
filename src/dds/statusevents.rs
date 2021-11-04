//
// Describe the commnucation status changes as events.
//
// These implement a mechanism equivalent to what is described in
// Section 2.2.4 Listeners, Conditions, and Wait-sets
//
// Communcation statues are detailed in Figure 2.13 and tables in Section
// 2.2.4.1 in DDS Specification v1.4

use mio::Evented;
use mio_extras::channel as mio_channel;

use crate::dds::qos::QosPolicyId;

/// This trait corresponds to set_listener() of the Entity class in DDS spec.
/// Types implementing this trait can be registered to a poll and
/// polled for status events.
pub trait StatusEvented<E> {
  fn as_status_evented(&mut self) -> &dyn Evented;
  fn try_recv_status(&self) -> Option<E>;
}

// Helper object for various DDS Entities
pub(crate) struct StatusReceiver<E> {
  channel_receiver: mio_channel::Receiver<E>,
  enabled: bool, // if not enabled, we should forward status to parent Entity
}

impl<E> StatusReceiver<E> {
  pub fn new(channel_receiver: mio_channel::Receiver<E>) -> StatusReceiver<E> {
    StatusReceiver::<E> {
      channel_receiver,
      enabled: false,
    }
  }
}

impl<E> StatusEvented<E> for StatusReceiver<E> {
  fn as_status_evented(&mut self) -> &dyn Evented {
    self.enabled = true;
    &self.channel_receiver
  }

  fn try_recv_status(&self) -> Option<E> {
    if self.enabled {
      self.channel_receiver.try_recv().ok()
    } else {
      None
    }
  }
}

#[derive(Debug, Clone)]
pub enum DomainParticipantStatus {
  PublisherStatus(PublisherStatus),
  SubscriberStatus(SubscriberStatus),
  TopicStatus(TopicStatus),
}

#[derive(Debug, Clone)]
pub enum SubscriberStatus {
  DataOnReaders,
  DataReaderStatus(DataReaderStatus),
}

pub type PublisherStatus = DataWriterStatus;

#[derive(Debug, Clone)]
pub enum TopicStatus {
  InconsistentTopic { count: CountWithChange },
}

#[derive(Debug, Clone)]
pub enum DataReaderStatus {
  /// Sample was rejected, because resource limits would have been exeeded.
  SampleRejected {
    count: CountWithChange,
    last_reason: SampleRejectedStatusKind,
    //last_instance_key:
  },
  /// Remote Writer has become active or inactive.
  LivelinessChanged {
    alive_total: CountWithChange,
    not_alive_total: CountWithChange,
    //last_publication_key:
  },
  /// Deadline requested by this DataReader was missed.
  RequestedDeadlineMissed {
    count: CountWithChange,
    //last_instance_key:
  },
  /// This DataReader has requested a QoS policy that is incompatibel with what
  /// is offered.
  RequestedIncompatibleQos {
    count: CountWithChange,
    last_policy_id: QosPolicyId,
    policies: Vec<QosPolicyCount>,
  },
  // DataAvailable is not implemented, as it seems to bring little additional value,
  // because the normal data waiting mechanism already uses the same mio::poll structure,
  // so repeating the functionality here would bring little additional value.
  /// A sample has been lost (never received).
  /// (Whtever this means?)
  SampleLost { count: CountWithChange },

  /// The DataReader has found a DataWriter that matches the Topic and has
  /// compatible QoS, or has ceased to be matched with a DataWriter that was
  /// previously considered to be matched.
  SubscriptionMatched {
    total: CountWithChange,
    current: CountWithChange,
    //last_publication_key:
  },
}

#[derive(Debug, Clone)]
pub enum DataWriterStatus {
  LivelinessLost {
    count: CountWithChange,
  },
  OfferedDeadlineMissed {
    count: CountWithChange,
    //last_instance_key:
  },
  OfferedIncompatibleQos {
    count: CountWithChange,
    last_policy_id: QosPolicyId,
    policies: Vec<QosPolicyCount>,
  },
  PublicationMatched {
    total: CountWithChange,
    current: CountWithChange,
    //last_subscription_key:
  },
}

/// Helper to contain same count actions across statuses
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CountWithChange {
  // 2.3. Platform Specific Model defines these as "long", which appears to be 32-bit signed.
  count: i32,
  count_change: i32,
}

impl CountWithChange {
  pub(crate) fn new(count: i32, count_change: i32) -> CountWithChange {
    CountWithChange {
      count,
      count_change,
    }
  }

  // ??
  // same as "new" ?
  pub fn start_from(count: i32, count_change: i32) -> CountWithChange {
    CountWithChange {
      count,
      count_change,
    }
  }

  pub fn count(&self) -> i32 {
    self.count
  }

  pub fn count_change(&self) -> i32 {
    self.count_change
  }

  // does this make sense?
  // pub fn increase(&mut self) {
  //   self.count += 1;
  //   self.count_change += 1;
  // }
}

// sample rejection reasons
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SampleRejectedStatusKind {
  NotRejected,
  ByInstancesLimit,
  BySamplesLimit,
  BySamplesPerInstanceLimit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QosPolicyCount {
  policy_id: QosPolicyId,
  count: i32,
}
