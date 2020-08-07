use crate::structure::cache_change::CacheChange;
use crate::structure::sequence_number::SequenceNumber;
use crate::structure::instance_handle::InstanceHandle;

use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct HistoryCache {
  changes: HashMap<InstanceHandle, CacheChange>,
}

impl HistoryCache {
  pub fn new() -> HistoryCache {
    HistoryCache {
      changes: HashMap::new(),
    }
  }

  pub fn add_change(&mut self, change: CacheChange) {
    self.changes.insert(change.instance_handle.clone(), change);
    // self.changes.push(change)
  }

  pub fn get_change(&self, sequence_number: SequenceNumber) -> Option<&CacheChange> {
    self
      .changes
      .iter()
      .find(|(_, x)| x.sequence_number == sequence_number)
      .map(|(_, x)| x)
  }

  pub fn remove_change(&mut self, sequence_number: SequenceNumber) {
    self
      .changes
      .retain(|_, x| x.sequence_number != sequence_number)
  }

  pub fn get_seq_num_min(&self) -> Option<&SequenceNumber> {
    self
      .changes
      .iter()
      .map(|(_, x)| &x.sequence_number)
      .min_by(|x, y| x.cmp(&y))
  }

  pub fn get_seq_num_max(&self) -> Option<&SequenceNumber> {
    self
      .changes
      .iter()
      .map(|(_, x)| &x.sequence_number)
      .max_by(|x, y| x.cmp(&y))
  }

  pub fn remove_changes_up_to(&mut self, smallest_seqnum: SequenceNumber) {
    self
      .changes
      .retain(|_, x| x.sequence_number > smallest_seqnum)
  }

  pub fn len(&self) -> usize {
    self.changes.len()
  }

  pub fn get_latest(&self) -> Option<&CacheChange> {
    self
      .changes
      .iter()
      .map(|(_, x)| x)
      .max_by(|x, y| x.sequence_number.cmp(&y.sequence_number))
  }

  pub fn generate_free_instance_handle(&self) -> InstanceHandle {
    let mut instance_handle = InstanceHandle::generate_random_key();
    while self.changes.contains_key(&instance_handle) {
      instance_handle = InstanceHandle::generate_random_key();
    }
    instance_handle
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::structure::guid::EntityId;
  use crate::structure::guid::GuidPrefix;
  use crate::structure::cache_change::ChangeKind;
  use crate::structure::guid::GUID;
  use crate::structure::instance_handle::InstanceHandle;

  use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
  use std::sync::Arc;

  #[test]
  fn ch_add_change_test() {
    let mut history_cache = HistoryCache::new();
    let cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: InstanceHandle::default(),
      sequence_number: SequenceNumber::SEQUENCENUMBER_UNKNOWN,
      data_value: Some(Arc::new(SerializedPayload::new())),
    };

    assert_eq!(0, history_cache.changes.len());

    history_cache.add_change(cache_change);
    assert_eq!(1, history_cache.changes.len());
  }

  #[test]
  fn ch_remove_change_test() {
    let mut history_cache = HistoryCache::new();

    assert_eq!(0, history_cache.changes.len());

    let cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: history_cache.generate_free_instance_handle(),
      sequence_number: SequenceNumber::from(10),
      data_value: Some(Arc::new(SerializedPayload::new())),
    };
    history_cache.add_change(cache_change);
    assert_eq!(1, history_cache.changes.len());

    let cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: history_cache.generate_free_instance_handle(),
      sequence_number: SequenceNumber::from(7),
      data_value: Some(Arc::new(SerializedPayload::new())),
    };
    history_cache.add_change(cache_change);
    assert_eq!(2, history_cache.changes.len());

    history_cache.remove_change(SequenceNumber::from(7));
    assert_eq!(1, history_cache.changes.len());
  }

  #[test]
  fn ch_get_seq_num_min() {
    let mut history_cache = HistoryCache::new();

    let small_cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: history_cache.generate_free_instance_handle(),
      sequence_number: SequenceNumber::from(1),
      data_value: Some(Arc::new(SerializedPayload::new())),
    };
    history_cache.add_change(small_cache_change);

    let big_cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: history_cache.generate_free_instance_handle(),
      sequence_number: SequenceNumber::from(7),
      data_value: Some(Arc::new(SerializedPayload::new())),
    };
    history_cache.add_change(big_cache_change);

    let smalles_cache_change = history_cache.get_seq_num_min();

    assert_eq!(true, smalles_cache_change.is_some());
    assert_eq!(&SequenceNumber::from(1), smalles_cache_change.unwrap());
  }

  #[test]
  fn ch_get_seq_num_max() {
    let mut history_cache = HistoryCache::new();

    let small_cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: InstanceHandle::default(),
      sequence_number: SequenceNumber::from(1),
      data_value: Some(Arc::new(SerializedPayload::new())),
    };
    history_cache.add_change(small_cache_change);

    let big_cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID {
        entityId: EntityId::ENTITYID_UNKNOWN,
        guidPrefix: GuidPrefix {
          entityKey: [0x00; 12],
        },
      },
      instance_handle: InstanceHandle::default(),
      sequence_number: SequenceNumber::from(7),
      data_value: Some(Arc::new(SerializedPayload::new())),
    };
    history_cache.add_change(big_cache_change);

    let biggest_cache_change = history_cache.get_seq_num_max();

    assert_eq!(true, biggest_cache_change.is_some());
    assert_eq!(&SequenceNumber::from(7), biggest_cache_change.unwrap());
  }
}
