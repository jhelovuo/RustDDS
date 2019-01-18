use crate::behavior::change_for_reader::ChangeForReader;
use crate::structure::guid::GUID_t;
use crate::structure::history_cache::CacheChange;
use crate::structure::locator::Locator_t;
use crate::structure::sequence_number::SequenceNumber_t;

/// The RTPS ReaderProxy class represents the information an RTPS StatefulWriter
/// maintains on each matched RTPS.
pub struct ReaderProxy {
    /// Identifies the remote matched RTPS Reader that is represented by the
    /// ReaderProxy.
    remoteReaderGuid: GUID_t,

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

    /// List of CacheChange changes as they  to the matched RTPS Reader.
    changes_for_reader: Vec<CacheChange>,

    /// Specifies whether the remote matched RTPS Reader expects in-line QoS to
    /// be sent along with any data
    expectsInlineQos: bool,

    /// Specifies whether the remote Reader is responsive to the Writer.
    isActive: bool,
}

impl ReaderProxy {
    pub fn new() -> Self {
        unimplemented!();
    }

    pub fn acked_changes_set(&mut self, committed_seq_num: SequenceNumber_t) {
        unimplemented!();
    }

    pub fn next_requested_change(&self) -> ChangeForReader {
        unimplemented!();
    }

    pub fn next_unsent_change(&self) -> ChangeForReader {
        unimplemented!();
    }

    pub fn unsent_changes(&self) -> Vec<ChangeForReader> {
        unimplemented!();
    }

    pub fn requested_changes(&self) -> Vec<ChangeForReader> {
        unimplemented!();
    }

    pub fn requested_changes_set(&self, req_seq_num_set: &[SequenceNumber_t]) {
        unimplemented!();
    }

    pub fn unacked_changes(&self) -> Vec<ChangeForReader> {
        unimplemented!();
    }
}
