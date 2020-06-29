use crate::structure::cache_change::CacheChange;
use crate::structure::sequence_number::SequenceNumber;

#[derive(Debug, PartialEq)]
pub struct HistoryCache {
  changes: Vec<CacheChange>,
}

impl HistoryCache {
  fn new() -> HistoryCache {
    HistoryCache {
      changes: Vec::new(),
    }
  }

  fn add_change(&mut self, change: CacheChange) {
    self.changes.push(change)
  }

  fn get_change(&self, sequence_number: SequenceNumber) -> Option<&CacheChange> {
    self
      .changes
      .iter()
      .find(|x| x.sequence_number == sequence_number)
  }

  fn remove_change(&mut self, sequence_number: SequenceNumber) {
    self
      .changes
      .retain(|x| x.sequence_number != sequence_number)
  }

  fn get_seq_num_min(&self) -> Option<&SequenceNumber> {
    self
      .changes
      .iter()
      .map(|x| &x.sequence_number)
      .min_by(|x, y| x.cmp(&y))
  }

  fn get_seq_num_max(&self) -> Option<&SequenceNumber> {
    self
      .changes
      .iter()
      .map(|x| &x.sequence_number)
      .max_by(|x, y| x.cmp(&y))
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
  use crate::structure::cache_change::Data;

  #[test]
  fn add_change_test() {
    let mut history_cache = HistoryCache::new();
    let cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: InstanceHandle::default(),
      sequence_number: SequenceNumber::SEQUENCENUMBER_UNKNOWN,
      data_value: Some(Data {}),
    };

    assert_eq!(0, history_cache.changes.len());

    history_cache.add_change(cache_change);
    assert_eq!(1, history_cache.changes.len());
  }

  #[test]
  fn remove_change_test() {
    let mut history_cache = HistoryCache::new();

    assert_eq!(0, history_cache.changes.len());

    let cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: InstanceHandle::default(),
      sequence_number: SequenceNumber::from(10),
      data_value: Some(Data {}),
    };
    history_cache.add_change(cache_change);
    assert_eq!(1, history_cache.changes.len());

    let cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: InstanceHandle::default(),
      sequence_number: SequenceNumber::from(7),
      data_value: Some(Data {}),
    };
    history_cache.add_change(cache_change);
    assert_eq!(2, history_cache.changes.len());

    history_cache.remove_change(SequenceNumber::from(7));
    assert_eq!(1, history_cache.changes.len());
  }

  #[test]
  fn get_seq_num_min() {
    let mut history_cache = HistoryCache::new();

    let small_cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: InstanceHandle::default(),
      sequence_number: SequenceNumber::from(1),
      data_value: Some(Data {}),
    };
    history_cache.add_change(small_cache_change);

    let big_cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: InstanceHandle::default(),
      sequence_number: SequenceNumber::from(7),
      data_value: Some(Data {}),
    };
    history_cache.add_change(big_cache_change);

    let smalles_cache_change = history_cache.get_seq_num_min();

    assert_eq!(true, smalles_cache_change.is_some());
    assert_eq!(&SequenceNumber::from(1), smalles_cache_change.unwrap());
  }

  #[test]
  fn get_seq_num_max() {
    let mut history_cache = HistoryCache::new();

    let small_cache_change = CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid: GUID::GUID_UNKNOWN,
      instance_handle: InstanceHandle::default(),
      sequence_number: SequenceNumber::from(1),
      data_value: Some(Data {}),
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
      data_value: Some(Data {}),
    };
    history_cache.add_change(big_cache_change);

    let biggest_cache_change = history_cache.get_seq_num_max();

    assert_eq!(true, biggest_cache_change.is_some());
    assert_eq!(&SequenceNumber::from(7), biggest_cache_change.unwrap());
  }
}
