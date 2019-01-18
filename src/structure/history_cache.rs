use crate::structure::change_kind::ChangeKind_t;
use crate::structure::guid::GUID_t;
use crate::structure::instance_handle::InstanceHandle_t;
use crate::structure::sequence_number::SequenceNumber_t;

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
struct Data {}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct CacheChange {
    kind: ChangeKind_t,
    writerGuid: GUID_t,
    instanceHandle: InstanceHandle_t,
    sequenceNumber: SequenceNumber_t,
    data_value: Data,
}

struct HistoryCache {
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

    fn get_change(&self, sequenceNumber: SequenceNumber_t) -> Option<&CacheChange> {
        self.changes
            .iter()
            .find(|x| x.sequenceNumber == sequenceNumber)
    }

    fn remove_change(&mut self, sequenceNumber: SequenceNumber_t) {
        self.changes.retain(|x| x.sequenceNumber != sequenceNumber)
    }

    fn get_seq_num_min(&self) -> Option<&SequenceNumber_t> {
        self.changes
            .iter()
            .map(|x| &x.sequenceNumber)
            .min_by(|x, y| x.cmp(&y))
    }

    fn get_seq_num_max(&self) -> Option<&SequenceNumber_t> {
        self.changes
            .iter()
            .map(|x| &x.sequenceNumber)
            .max_by(|x, y| x.cmp(&y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::entity_id::EntityId_t;
    use crate::structure::guid_prefix::GuidPrefix_t;

    #[test]
    fn add_change_test() {
        let mut history_cache = HistoryCache::new();
        let cache_change = CacheChange {
            kind: ChangeKind_t::ALIVE,
            writerGuid: GUID_t::GUID_UNKNOWN,
            instanceHandle: InstanceHandle_t::default(),
            sequenceNumber: SequenceNumber_t::SEQUENCENUMBER_UNKNOWN,
            data_value: Data {},
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
            kind: ChangeKind_t::ALIVE,
            writerGuid: GUID_t::GUID_UNKNOWN,
            instanceHandle: InstanceHandle_t::default(),
            sequenceNumber: SequenceNumber_t { value: 10 },
            data_value: Data {},
        };
        history_cache.add_change(cache_change);
        assert_eq!(1, history_cache.changes.len());

        let cache_change = CacheChange {
            kind: ChangeKind_t::ALIVE,
            writerGuid: GUID_t::GUID_UNKNOWN,
            instanceHandle: InstanceHandle_t::default(),
            sequenceNumber: SequenceNumber_t { value: 7 },
            data_value: Data {},
        };
        history_cache.add_change(cache_change);
        assert_eq!(2, history_cache.changes.len());

        history_cache.remove_change(SequenceNumber_t { value: 7 });
        assert_eq!(1, history_cache.changes.len());
    }

    #[test]
    fn get_seq_num_min() {
        let mut history_cache = HistoryCache::new();

        let small_cache_change = CacheChange {
            kind: ChangeKind_t::ALIVE,
            writerGuid: GUID_t::GUID_UNKNOWN,
            instanceHandle: InstanceHandle_t::default(),
            sequenceNumber: SequenceNumber_t { value: 1 },
            data_value: Data {},
        };
        history_cache.add_change(small_cache_change);

        let big_cache_change = CacheChange {
            kind: ChangeKind_t::ALIVE,
            writerGuid: GUID_t::GUID_UNKNOWN,
            instanceHandle: InstanceHandle_t::default(),
            sequenceNumber: SequenceNumber_t { value: 7 },
            data_value: Data {},
        };
        history_cache.add_change(big_cache_change);

        let smalles_cache_change = history_cache.get_seq_num_min();

        assert_eq!(true, smalles_cache_change.is_some());
        assert_eq!(
            &SequenceNumber_t { value: 1 },
            smalles_cache_change.unwrap()
        );
    }

    #[test]
    fn get_seq_num_max() {
        let mut history_cache = HistoryCache::new();

        let small_cache_change = CacheChange {
            kind: ChangeKind_t::ALIVE,
            writerGuid: GUID_t::GUID_UNKNOWN,
            instanceHandle: InstanceHandle_t::default(),
            sequenceNumber: SequenceNumber_t { value: 1 },
            data_value: Data {},
        };
        history_cache.add_change(small_cache_change);

        let big_cache_change = CacheChange {
            kind: ChangeKind_t::ALIVE,
            writerGuid: GUID_t {
                entityId: EntityId_t::ENTITYID_UNKNOWN,
                guidPrefix: GuidPrefix_t {
                    entityKey: [0x00; 12],
                },
            },
            instanceHandle: InstanceHandle_t::default(),
            sequenceNumber: SequenceNumber_t { value: 7 },
            data_value: Data {},
        };
        history_cache.add_change(big_cache_change);

        let biggest_cache_change = history_cache.get_seq_num_max();

        assert_eq!(true, biggest_cache_change.is_some());
        assert_eq!(
            &SequenceNumber_t { value: 7 },
            biggest_cache_change.unwrap()
        );
    }
}
