use crate::behavior::reader_proxy::ReaderProxy;
use crate::structure::guid::GUID_t;
use crate::structure::history_cache::CacheChange;

pub struct StatefulWriter {
    /// The StatefulWriter keeps track of all the RTPS Readers matched with it.
    /// Each matched reader is represented by an instance of the ReaderProxy
    /// class.
    matched_readers: Vec<ReaderProxy>,
}

impl StatefulWriter {
    pub fn new() -> Self {
        unimplemented!();
    }

    pub fn matched_reader_add(&mut self, a_reader_proxy: ReaderProxy) {
        unimplemented!();
    }

    pub fn matched_reader_remove(&mut self, a_reader_proxy: ReaderProxy) {
        unimplemented!();
    }

    pub fn matched_reader_lookup(&mut self, a_reader_guid: GUID_t) {
        unimplemented!();
    }

    pub fn is_acked_by_all(&mut self, a_change: CacheChange) {
        unimplemented!();
    }
}
