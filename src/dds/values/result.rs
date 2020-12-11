use std::result;

// This is a specialized Result, similar to std::io::Result
pub type Result<T> = result::Result<T, Error>;

/// This roughly corresponds to "Return codes" in DDS spec 2.2.1.1 Format and Conventions
#[derive(Debug)]
pub enum Error {
  // OK is not included. It is not an error. Ok/Error shoudl be distinguished with the Result type.
  // Error, // unspecified, please do not use these
  BadParameter,
  Unsupported,
  // AlreadyDeleted, // we should use Rust type system to avoid these, so no need for run-time error.
  OutOfResources,
  NotEnabled,
  ImmutablePolicy, // can we check this statically?
  InconsistentPolicy,
  PreconditionNotMet,
  //Timeout,  // this is normal operation and should be encoded as Option<> or Result<>
  IllegalOperation,
  //NoData,  // this should be encoded as Option<SomeData>, not an error code

  /// Synchronization with another thread failed because the other thread
  /// has exited while holding a lock.
  /// [https://doc.rust-lang.org/std/sync/struct.PoisonError.html]
  LockPoisoned,  
}

/// Helper to contain same count actions across statuses
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct CountWithChange {
  count: i32,
  count_change: i32,
}

impl CountWithChange {
  pub(crate) fn new() -> CountWithChange {
    CountWithChange {
      count: 0,
      count_change: 0,
    }
  }

  #[cfg(test)]
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

  pub fn increase(&mut self) {
    self.count += 1;
    self.count_change += 1;
  }

  pub fn reset_count(&mut self) {
    self.count_change = 0;
  }
}

/// DDS InconsistentTopicStatus
pub struct InconsistentTopicStatus {
  total: CountWithChange,
}

impl InconsistentTopicStatus {
  /// Total cumulative count of the Topics discovered whose name matches the Topic to which this status is attached and whose type is inconsistent with the Topic.
  pub fn count(&self) -> i32 {
    self.total.count()
  }

  /// The incremental number of inconsistent topics discovered since the last time the listener was called or the status was read.
  pub fn count_change(&self) -> i32 {
    self.total.count_change()
  }
}

/// DDS SampleLostStatus
pub struct SampleLostStatus {
  total: CountWithChange,
}

impl SampleLostStatus {
  /// Total cumulative count of all samples lost across of instances of data published under the Topic.
  pub fn count(&self) -> i32 {
    self.total.count()
  }

  /// The incremental number of samples lost since the last time the listener was called or the status was read.
  pub fn count_change(&self) -> i32 {
    self.total.count_change()
  }
}

/// Reason for sample rejection
#[derive(Clone, Copy)]
pub enum SampleRejectedReason {
  InstancesLimit,
  SamplesLimit,
  SamplesPerInstanceLimit,
}

/// DDS SampleRejectedStatus
pub struct SampleRejectedStatus {
  total: CountWithChange,
  last_reason: Option<SampleRejectedReason>, // None == NOT_REJECTED
                                             // missing: last_instance_handle: instance key indicating last rejected instance
}

impl SampleRejectedStatus {
  /// Total cumulative count of samples rejected by the DataReader.
  pub fn count(&self) -> i32 {
    self.total.count()
  }

  /// The incremental number of samples rejected since the last time the listener was called or the status was read.
  pub fn count_change(&self) -> i32 {
    self.total.count_change()
  }

  /// Reason for rejecting the last sample rejected. If no samples have been rejected, the reason is None.
  pub fn sample_rejected_reason(&self) -> Option<SampleRejectedReason> {
    self.last_reason
  }
}

/// All possible status changes
#[derive(Debug, Clone)]
pub enum StatusChange {
  LivelinessLostStatus(LivelinessLostStatus),
  OfferedDeadlineMissedStatus(OfferedDeadlineMissedStatus),
  OfferedIncompatibleQosStatus(OfferedIncompatibleQosStatus),
  RequestedDeadlineMissedStatus(RequestedDeadlineMissedStatus),
  RequestedIncompatibleQosStatus(RequestedIncompatibleQosStatus),
  PublicationMatchedStatus(PublicationMatchedStatus),
  SubscriptionMatchedStatus(SubscriptionMatchedStatus),
}

/// DDS LivelinessLostStatus
#[derive(Debug, Copy, Clone)]
pub struct LivelinessLostStatus {
  total: CountWithChange,
}

impl LivelinessLostStatus {
  /// Total cumulative number of times that a previously-alive DataWriter became not alive due to a failure to actively signal its liveliness within its offered liveliness period.
  /// This count does not change when an already not alive DataWriter simply remains not alive for another liveliness period.
  pub fn count(&self) -> i32 {
    self.total.count()
  }

  /// The change in total_count since the last time the listener was called or the status was read.
  pub fn count_change(&self) -> i32 {
    self.total.count_change()
  }
}

/// DDS OfferedDeadlineMissedStatus
#[derive(Debug, Copy, Clone)]
pub struct OfferedDeadlineMissedStatus {
  total: CountWithChange,
  // missing: last instance key
}

impl OfferedDeadlineMissedStatus {
  pub(crate) fn new() -> OfferedDeadlineMissedStatus {
    OfferedDeadlineMissedStatus {
      total: CountWithChange {
        count: 0,
        count_change: 0,
      },
    }
  }

  /// Total cumulative number of offered deadline periods elapsed during which
  /// a DataWriter failed to provide data. Missed deadlines accumulate; that
  /// is, each deadline period the total_count will be incremented by one.
  pub fn count(&self) -> i32 {
    self.total.count()
  }

  /// The change in total_count since the last time the listener was called
  /// or the status was read.
  pub fn count_change(&self) -> i32 {
    self.total.count_change()
  }

  pub(crate) fn increase(&mut self) {
    self.total.increase();
  }

  pub(crate) fn reset_change(&mut self) {
    self.total.reset_count();
  }
}

/// DDS OfferedIncompatibleQosStatus
#[derive(Debug, Clone)]
pub struct OfferedIncompatibleQosStatus {
  total: CountWithChange,
  //TODO: last_policy_id: QosPolicyId_t
  //TODO: policies: QosPolicyCountSeq
}

impl OfferedIncompatibleQosStatus {
  /// Total cumulative number of times the concerned DataWriter discovered a
  /// DataReader for the same Topic with a requested QoS that is incompatible
  /// with that offered by the DataWriter.
  pub fn count(&self) -> i32 {
    self.total.count()
  }

  pub fn count_change(&self) -> i32 {
    self.total.count_change()
  }
}

/// DDS RequestedDeadlineMissedStatus
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RequestedDeadlineMissedStatus {
  total: CountWithChange,
  // missing: last instance handle
}

impl RequestedDeadlineMissedStatus {
  pub(crate) fn new() -> RequestedDeadlineMissedStatus {
    RequestedDeadlineMissedStatus {
      total: CountWithChange {
        count: 0,
        count_change: 0,
      },
    }
  }

  #[cfg(test)]
  pub(crate) fn from_count(total: CountWithChange) -> RequestedDeadlineMissedStatus {
    RequestedDeadlineMissedStatus { total }
  }

  /// Total cumulative number of missed deadlines detected for any instance
  /// read by the DataReader. Missed deadlines accumulate; that is, each
  /// deadline period the total_count will be incremented by one for each
  /// instance for which data was not received.
  pub fn count(&self) -> i32 {
    self.total.count()
  }

  /// The incremental number of deadlines detected since the last time the
  /// listener was called or the status was read.
  pub fn count_change(&self) -> i32 {
    self.total.count_change()
  }

  pub(crate) fn increase(&mut self) {
    self.total.increase();
  }

  pub(crate) fn reset_change(&mut self) {
    self.total.reset_count();
  }
}

/// DDS RequestedIncompatibleQosStatus
#[derive(Debug, Clone)]
pub struct RequestedIncompatibleQosStatus {
  total: CountWithChange,
  //TODO: last_policy_id: QosPolicyId_t
  //TODO: policies: QosPolicyCountSeq
}

impl RequestedIncompatibleQosStatus {
  /// Total cumulative number of times the concerned DataReader discovered a
  /// DataWriter for the same Topic with an offered QoS that was incompatible
  /// with that requested by the DataReader.
  pub fn count(&self) -> i32 {
    self.total.count()
  }

  /// The change in total_count since the last time the listener was called or
  /// the status was read
  pub fn count_change(&self) -> i32 {
    self.total.count_change()
  }
}

/// DDS PublicationMatchedStatus
#[derive(Debug, Copy, Clone)]
pub struct PublicationMatchedStatus {
  total: CountWithChange,
  current: CountWithChange,
  // Missing: reference to last instance key
}

impl PublicationMatchedStatus {
  /// Total cumulative count the concerned DataWriter discovered a “match” with
  /// a DataReader. That is, it found a DataReader for the same Topic with a
  /// requested QoS that is compatible with that offered by the DataWriter.
  pub fn total_count(&self) -> i32 {
    self.total.count()
  }

  /// The change in total_count since the last time the listener was called or
  /// the status was read.
  pub fn total_count_change(&self) -> i32 {
    self.total.count_change()
  }

  /// The number of DataReaders currently matched to the concerned DataWriter.
  pub fn current_count(&self) -> i32 {
    self.current.count()
  }

  /// The change in current_count since the last time the listener was called
  /// or the status was read.  
  pub fn current_count_change(&self) -> i32 {
    self.current.count_change()
  }
}

/// DDS SubscriptionMatchedStatus
#[derive(Debug, Copy, Clone)]
pub struct SubscriptionMatchedStatus {
  total: CountWithChange,
  current: CountWithChange,
  // Missing: reference to last instance key
}

impl SubscriptionMatchedStatus {
  /// Total cumulative count the concerned DataReader discovered a “match”
  /// with a DataWriter. That is, it found a DataWriter for the same Topic with
  /// a requested QoS that is compatible with that offered by the DataReader.
  pub fn total_count(&self) -> i32 {
    self.total.count()
  }

  /// The change in total_count since the last time the listener was called or
  /// the status was read.  
  pub fn total_count_change(&self) -> i32 {
    self.total.count_change()
  }

  /// The number of DataWriters currently matched to the concerned DataReader.
  pub fn current_count(&self) -> i32 {
    self.current.count()
  }

  /// The change in current_count since the last time the listener was called
  /// or the status was read.
  pub fn current_count_change(&self) -> i32 {
    self.current.count_change()
  }
}
