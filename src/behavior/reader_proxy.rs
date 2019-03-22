use crate::behavior::change_for_reader::ChangeForReader;
use crate::behavior::change_for_reader_status_kind::ChangeForReaderStatusKind;
use crate::structure::cache_change::CacheChange;
use crate::structure::guid::GUID_t;
use crate::structure::locator::Locator_t;
use crate::structure::sequence_number::SequenceNumber_t;

/// The RTPS ReaderProxy class represents the information an RTPS StatefulWriter
/// maintains on each matched RTPS.
pub struct ReaderProxy {
    /// Identifies the remote matched RTPS Reader that is represented by the
    /// ReaderProxy.
    pub remote_reader_guid: GUID_t,

    /// Specifies whether the remote matched RTPS Reader expects in-line QoS to
    /// be sent along with any data
    expects_inline_qos: bool,

    /// Specifies whether the remote Reader is responsive to the Writer.
    is_active: bool,

    highest_seq_num_sent: SequenceNumber_t,
}

impl ReaderProxy {
    pub fn new(remote_reader_guid: GUID_t, expects_inline_qos: bool) -> ReaderProxy {
        ReaderProxy {
            remote_reader_guid,
            expects_inline_qos,
            is_active: true,
            highest_seq_num_sent: SequenceNumber_t::from(std::i64::MIN),
        }
    }

    pub fn next_unsent_change<'a>(
        &'a mut self,
        changes: &'a [CacheChange],
    ) -> Option<&'a CacheChange> {
        self.unsent_changes(&changes)
            .min_by(|x, y| x.sequence_number.cmp(&y.sequence_number))
            .and_then(|change| {
                self.highest_seq_num_sent = change.sequence_number;
                Some(change)
            })
    }

    pub fn unsent_changes<'a>(
        &self,
        changes: &'a [CacheChange],
    ) -> impl Iterator<Item = &'a CacheChange> {
        let highest_seq_num_sent = self.highest_seq_num_sent;
        changes
            .iter()
            .filter(move |change| change.sequence_number > highest_seq_num_sent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::change_kind::ChangeKind_t;
    use crate::structure::data::Data;
    use crate::structure::instance_handle::InstanceHandle_t;

    fn default_cache_change(sequence_number: i64) -> CacheChange {
        CacheChange {
            kind: ChangeKind_t::ALIVE,
            writer_guid: GUID_t::GUID_UNKNOWN,
            instance_handle: InstanceHandle_t::default(),
            sequence_number: SequenceNumber_t::from(sequence_number),
            data_value: Data {},
        }
    }

    #[test]
    fn unsent_changes_returns_not_consumed_changes_by_default() {
        let reader_proxy = ReaderProxy::new(GUID_t::GUID_UNKNOWN, true);

        let changes = vec![
            default_cache_change(0),
            default_cache_change(1),
            default_cache_change(2),
        ];

        let mut unsent_changes = reader_proxy.unsent_changes(&changes);
        assert_eq!(Some(&changes[0]), unsent_changes.next());
        assert_eq!(Some(&changes[1]), unsent_changes.next());
        assert_eq!(Some(&changes[2]), unsent_changes.next());
        assert_eq!(None, unsent_changes.next());
    }

    #[test]
    fn next_unsent_change_returns_cache_change_with_smallest_sequence_number() {
        let mut reader_proxy = ReaderProxy::new(GUID_t::GUID_UNKNOWN, true);

        let changes = vec![
            default_cache_change(6),
            default_cache_change(1),
            default_cache_change(3),
        ];

        assert_eq!(Some(&changes[1]), reader_proxy.next_unsent_change(&changes));
        assert_eq!(Some(&changes[2]), reader_proxy.next_unsent_change(&changes));
        assert_eq!(Some(&changes[0]), reader_proxy.next_unsent_change(&changes));
        assert_eq!(None, reader_proxy.next_unsent_change(&changes));
    }

    #[test]
    fn next_unsent_change_once_requested_does_not_belong_to_unsent_changes() {
        let mut reader_proxy = ReaderProxy::new(GUID_t::GUID_UNKNOWN, true);

        let changes = vec![
            default_cache_change(6),
            default_cache_change(1),
            default_cache_change(3),
        ];

        assert_eq!(3, reader_proxy.unsent_changes(&changes).count());
        assert!(reader_proxy
            .unsent_changes(&changes)
            .any(|change| change == &changes[1]));

        let next_unsent_change = reader_proxy.next_unsent_change(&changes);

        assert_eq!(2, reader_proxy.unsent_changes(&changes).count());
        assert!(!reader_proxy
            .unsent_changes(&changes)
            .any(|change| change == &changes[1]));
    }

    #[test]
    fn unsent_changes_returns_only_changes_that_were_not_sent() {
        let mut reader_proxy = ReaderProxy::new(GUID_t::GUID_UNKNOWN, true);

        let mut changes = vec![
            default_cache_change(6),
            default_cache_change(1),
            default_cache_change(3),
        ];

        // consume all changes
        reader_proxy.next_unsent_change(&changes);
        reader_proxy.next_unsent_change(&changes);
        reader_proxy.next_unsent_change(&changes);

        // add new changes with smaller sequence numbers than highest sent
        changes.push(default_cache_change(0));
        changes.push(default_cache_change(5));
        changes.push(default_cache_change(3));

        assert_eq!(None, reader_proxy.next_unsent_change(&changes));

        // add new change with sequence number higher than highest sent
        changes.push(default_cache_change(10));

        assert_eq!(changes.last(), reader_proxy.next_unsent_change(&changes));
    }
}
