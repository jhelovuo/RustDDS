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
    pub remoteReaderGuid: GUID_t,

    /// List of unicast locators (transport, address, port combinations) that
    /// can be used to send messages to the matched RTPS Reader.
    ///
    /// The list may be empty.
    unicastLocatorList: Vec<Locator_t>,

    /// List of multicast locators (transport, address, port combinations) that
    /// can be used to send messages to the matched RTPS Reader.
    ///
    /// The list may be empty.
    multicastLocatorList: Vec<Locator_t>,

    /// List of CacheChange changes as they to the matched RTPS Reader.
    changes_for_reader: Vec<(CacheChange, ChangeForReader)>,

    /// Specifies whether the remote matched RTPS Reader expects in-line QoS to
    /// be sent along with any data
    expectsInlineQos: bool,

    /// Specifies whether the remote Reader is responsive to the Writer.
    isActive: bool,
}

impl ReaderProxy {
    pub fn new(
        remoteReaderGuid: GUID_t,
        expectsInlineQos: bool,
        unicastLocatorList: &[Locator_t],
        multicastLocatorList: &[Locator_t],
    ) -> ReaderProxy {
        unimplemented!();
    }

    pub fn acked_changes_set(&mut self, committed_seq_num: SequenceNumber_t) {
        self.changes_for_reader
            .iter_mut()
            .filter(move |change| change.0.sequenceNumber <= committed_seq_num)
            .for_each(|change| change.1.status = ChangeForReaderStatusKind::ACKNOWLEDGED);
    }

    pub fn next_requested_change(&self) -> &CacheChange {
        unimplemented!();
    }

    pub fn next_unsent_change(&self) -> &CacheChange {
        unimplemented!();
    }

    pub fn unsent_changes(&self) -> impl Iterator<Item = &CacheChange> {
        self.changes_for_reader
            .iter()
            .filter(|change| change.1.status == ChangeForReaderStatusKind::UNSENT)
            .map(|change| &change.0)
    }

    pub fn requested_changes(&self) -> impl Iterator<Item = &CacheChange> {
        self.changes_for_reader
            .iter()
            .filter(|change| change.1.status == ChangeForReaderStatusKind::REQUESTED)
            .map(|change| &change.0)
    }

    pub fn requested_changes_set(&mut self, req_seq_num_set: &[SequenceNumber_t]) {
        req_seq_num_set.iter().for_each(|seq_num| {
            if let Some(change_for_reader) = self
                .changes_for_reader
                .iter_mut()
                .find(|change_for_reader| change_for_reader.0.sequenceNumber == *seq_num)
            {
                change_for_reader.1.status = ChangeForReaderStatusKind::REQUESTED;
            }
        });
    }

    pub fn unacked_changes(&self) -> impl Iterator<Item = &CacheChange> {
        self.changes_for_reader
            .iter()
            .filter(|change| change.1.status == ChangeForReaderStatusKind::UNACKNOWLEDGED)
            .map(|change| &change.0)
    }
}
