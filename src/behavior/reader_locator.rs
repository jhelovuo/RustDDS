use crate::behavior::change_for_reader::ChangeForReader;
use crate::structure::cache_change::CacheChange;
use crate::structure::locator::Locator_t;
use crate::structure::sequence_number::SequenceNumber_t;

/// Valuetype used by the RTPS StatelessWriter to keep track
/// of the locators of all matching remote Readers
#[derive(Debug, PartialEq)]
pub struct ReaderLocator {
  /// A list of changes in the writer’s HistoryCache that
  /// were requested by remote Readers at this ReaderLocator
  requested_changes: Vec<CacheChange>,

  /// A list of changes in the writer’s HistoryCache that
  /// have not been sent yet to this ReaderLocator
  unsent_changes: Vec<CacheChange>,

  /// Unicast or multicast locator through which the readers
  /// represented by this ReaderLocator can be reached
  locator: Locator_t,

  /// Specifies whether the readers represented by this ReaderLocator
  /// expect inline QoS to be sent with every Data Message
  expectsInlineQos: bool,
}

impl ReaderLocator {
  pub fn new(_locator: Locator_t, _expectsInlineQos: bool) -> Self {
    unimplemented!();
  }

  pub fn next_requested_change() -> ChangeForReader {
    unimplemented!();
  }

  pub fn next_unset_change() -> ChangeForReader {
    unimplemented!();
  }

  pub fn requested_changes() -> Vec<CacheChange> {
    unimplemented!();
  }

  pub fn requested_changes_set(_req_seq_num_set: &[SequenceNumber_t]) {
    unimplemented!();
  }

  pub fn unset_changes() -> Vec<CacheChange> {
    unimplemented!();
  }
}
