use common::ChangeKind;
use common::EntityId;
use common::InstanceHandle;
use common::GuidPrefix;
use common::GUID;
use common::SequenceNumber;

#[derive(PartialOrd, PartialEq, Ord, Eq)]
struct Data {

}

#[derive(PartialOrd, PartialEq, Ord, Eq)]
struct CacheChange {
    kind: ChangeKind::ChangeKind_t,
    writerGuid: GUID::GUID_t,
    instanceHandle: InstanceHandle::InstanceHandle_t,
    sequenceNumber: SequenceNumber::SequenceNumber_t,
    data_value: Data
}

struct HistoryCache {
    changes: Vec<CacheChange>
}

impl HistoryCache {
    fn new() -> HistoryCache {
        HistoryCache {
            changes: Vec::new()
        }
    }

    fn add_change(&mut self, change: CacheChange) {
        self.changes.push(change)
    }

    fn get_change(&self, sequenceNumber: SequenceNumber::SequenceNumber_t) -> Option<&CacheChange> {
        self.changes.iter().find(|x| x.sequenceNumber == sequenceNumber)
    }

    fn remove_change(&mut self, sequenceNumber: SequenceNumber::SequenceNumber_t) {
        self.changes.retain(|x| x.sequenceNumber != sequenceNumber)
    }

    fn get_seq_num_min(&self) -> Option<&SequenceNumber::SequenceNumber_t> {
        self.changes.iter().map(|x| &x.sequenceNumber).min_by(|x, y| x.cmp(&y))
    }

    fn get_seq_num_max(&self) -> Option<&SequenceNumber::SequenceNumber_t> {
        self.changes.iter().map(|x| &x.sequenceNumber).max_by(|x, y| x.cmp(&y))
    }
}

#[test]
fn add_change_test() {
    let mut history_cache = HistoryCache::new();
    let cache_change = CacheChange {
        kind: ChangeKind::ChangeKind_t::ALIVE,
        writerGuid: GUID::GUID_UNKNOWN,
        instanceHandle: InstanceHandle::InstanceHandle_t {},
        sequenceNumber: SequenceNumber::SEQUENCENUMBER_UNKNOWN,
        data_value: Data {}
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
        kind: ChangeKind::ChangeKind_t::ALIVE,
        writerGuid: GUID::GUID_UNKNOWN,
        instanceHandle: InstanceHandle::InstanceHandle_t {},
        sequenceNumber: SequenceNumber::SequenceNumber_t { high: 5, low: 1 },
        data_value: Data {}
    };
    history_cache.add_change(cache_change);
    assert_eq!(1, history_cache.changes.len());

    let cache_change = CacheChange {
        kind: ChangeKind::ChangeKind_t::ALIVE,
        writerGuid: GUID::GUID_UNKNOWN,
        instanceHandle: InstanceHandle::InstanceHandle_t {},
        sequenceNumber: SequenceNumber::SequenceNumber_t { high: 7, low: 1 },
        data_value: Data {}
    };
    history_cache.add_change(cache_change);
    assert_eq!(2, history_cache.changes.len());

    history_cache.remove_change(SequenceNumber::SequenceNumber_t { high: 7, low: 1 });
    assert_eq!(1, history_cache.changes.len());
}

#[test]
fn get_seq_num_min() {
    let mut history_cache = HistoryCache::new();

    let small_cache_change = CacheChange {
        kind: ChangeKind::ChangeKind_t::ALIVE,
        writerGuid: GUID::GUID_UNKNOWN,
        instanceHandle: InstanceHandle::InstanceHandle_t {},
        sequenceNumber: SequenceNumber::SequenceNumber_t { high: 1, low: 1 },
        data_value: Data {}
    };
    history_cache.add_change(small_cache_change);

    let big_cache_change = CacheChange {
        kind: ChangeKind::ChangeKind_t::ALIVE,
        writerGuid: GUID::GUID_UNKNOWN,
        instanceHandle: InstanceHandle::InstanceHandle_t {},
        sequenceNumber: SequenceNumber::SequenceNumber_t { high: 7, low: 1 },
        data_value: Data {}
    };
    history_cache.add_change(big_cache_change);

    let smalles_cache_change = history_cache.get_seq_num_min();

    assert_eq!(true, smalles_cache_change.is_some());
    assert_eq!(&SequenceNumber::SequenceNumber_t { high: 1, low: 1 }, smalles_cache_change.unwrap());
}

#[test]
fn get_seq_num_max() {
    let mut history_cache = HistoryCache::new();

    let small_cache_change = CacheChange {
        kind: ChangeKind::ChangeKind_t::ALIVE,
        writerGuid: GUID::GUID_UNKNOWN,
        instanceHandle: InstanceHandle::InstanceHandle_t {},
        sequenceNumber: SequenceNumber::SequenceNumber_t { high: 1, low: 1 },
        data_value: Data {}
    };
    history_cache.add_change(small_cache_change);

    let big_cache_change = CacheChange {
        kind: ChangeKind::ChangeKind_t::ALIVE,
        writerGuid: GUID::GUID_t {
            entityId: EntityId::EntityId_t {
                entityKey: [0x00; 3],
                entityKind: 0
            },
            guidPrefix: GuidPrefix::GuidPrefix_t {
                entityKey: [0x00; 12]
            }
        },
        instanceHandle: InstanceHandle::InstanceHandle_t {},
        sequenceNumber: SequenceNumber::SequenceNumber_t { high: 7, low: 1 },
        data_value: Data {}
    };
    history_cache.add_change(big_cache_change);

    let biggest_cache_change = history_cache.get_seq_num_max();

    assert_eq!(true, biggest_cache_change.is_some());
    assert_eq!(&SequenceNumber::SequenceNumber_t { high: 7, low: 1 }, biggest_cache_change.unwrap());
}
