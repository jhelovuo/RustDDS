use common::change_kind;
use common::entity_id;
use common::instance_handle;
use common::guid_prefix;
use common::guid;
use common::sequence_number;

#[derive(PartialOrd, PartialEq, Ord, Eq)]
struct Data {

}

#[derive(PartialOrd, PartialEq, Ord, Eq)]
struct CacheChange {
    kind: change_kind::ChangeKind_t,
    writerGuid: guid::Guid_t,
    instanceHandle: instance_handle::InstanceHandle_t,
    sequenceNumber: sequence_number::SequenceNumber_t,
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

    fn get_change(&self, sequenceNumber: sequence_number::SequenceNumber_t) -> Option<&CacheChange> {
        self.changes.iter().find(|x| x.sequenceNumber == sequenceNumber)
    }

    fn remove_change(&mut self, sequenceNumber: sequence_number::SequenceNumber_t) {
        self.changes.retain(|x| x.sequenceNumber != sequenceNumber)
    }

    fn get_seq_num_min(&self) -> Option<&sequence_number::SequenceNumber_t> {
        self.changes.iter().map(|x| &x.sequenceNumber).min_by(|x, y| x.cmp(&y))
    }

    fn get_seq_num_max(&self) -> Option<&sequence_number::SequenceNumber_t> {
        self.changes.iter().map(|x| &x.sequenceNumber).max_by(|x, y| x.cmp(&y))
    }
}

#[test]
fn add_change_test() {
    let mut history_cache = HistoryCache::new();
    let cache_change = CacheChange {
        kind: change_kind::ChangeKind_t::ALIVE,
        writerGuid: guid::GUID_UNKNOWN,
        instanceHandle: instance_handle::InstanceHandle_t::default(),
        sequenceNumber: sequence_number::SEQUENCENUMBER_UNKNOWN,
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
        kind: change_kind::ChangeKind_t::ALIVE,
        writerGuid: guid::GUID_UNKNOWN,
        instanceHandle: instance_handle::InstanceHandle_t::default(),
        sequenceNumber: sequence_number::SequenceNumber_t { high: 5, low: 1 },
        data_value: Data {}
    };
    history_cache.add_change(cache_change);
    assert_eq!(1, history_cache.changes.len());

    let cache_change = CacheChange {
        kind: change_kind::ChangeKind_t::ALIVE,
        writerGuid: guid::GUID_UNKNOWN,
        instanceHandle: instance_handle::InstanceHandle_t::default(),
        sequenceNumber: sequence_number::SequenceNumber_t { high: 7, low: 1 },
        data_value: Data {}
    };
    history_cache.add_change(cache_change);
    assert_eq!(2, history_cache.changes.len());

    history_cache.remove_change(sequence_number::SequenceNumber_t { high: 7, low: 1 });
    assert_eq!(1, history_cache.changes.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_seq_num_min() {
        let mut history_cache = HistoryCache::new();

        let small_cache_change = CacheChange {
            kind: change_kind::ChangeKind_t::ALIVE,
            writerGuid: guid::GUID_UNKNOWN,
            instanceHandle: instance_handle::InstanceHandle_t::default(),
            sequenceNumber: sequence_number::SequenceNumber_t { high: 1, low: 1 },
            data_value: Data {}
        };
        history_cache.add_change(small_cache_change);

        let big_cache_change = CacheChange {
            kind: change_kind::ChangeKind_t::ALIVE,
            writerGuid: guid::GUID_UNKNOWN,
            instanceHandle: instance_handle::InstanceHandle_t::default(),
            sequenceNumber: sequence_number::SequenceNumber_t { high: 7, low: 1 },
            data_value: Data {}
        };
        history_cache.add_change(big_cache_change);

        let smalles_cache_change = history_cache.get_seq_num_min();

        assert_eq!(true, smalles_cache_change.is_some());
        assert_eq!(&sequence_number::SequenceNumber_t { high: 1, low: 1 }, smalles_cache_change.unwrap());
    }

    #[test]
    fn get_seq_num_max() {
        let mut history_cache = HistoryCache::new();

        let small_cache_change = CacheChange {
            kind: change_kind::ChangeKind_t::ALIVE,
            writerGuid: guid::GUID_UNKNOWN,
            instanceHandle: instance_handle::InstanceHandle_t::default(),
            sequenceNumber: sequence_number::SequenceNumber_t { high: 1, low: 1 },
            data_value: Data {}
        };
        history_cache.add_change(small_cache_change);

        let big_cache_change = CacheChange {
            kind: change_kind::ChangeKind_t::ALIVE,
            writerGuid: guid::Guid_t {
                entityId: entity_id::EntityId_t {
                    entityKey: [0x00; 3],
                    entityKind: 0
                },
                guidPrefix: guid_prefix::GuidPrefix_t {
                    entityKey: [0x00; 12]
                }
            },
            instanceHandle: instance_handle::InstanceHandle_t::default(),
            sequenceNumber: sequence_number::SequenceNumber_t { high: 7, low: 1 },
            data_value: Data {}
        };
        history_cache.add_change(big_cache_change);

        let biggest_cache_change = history_cache.get_seq_num_max();

        assert_eq!(true, biggest_cache_change.is_some());
        assert_eq!(&sequence_number::SequenceNumber_t { high: 7, low: 1 }, biggest_cache_change.unwrap());
    }
}
