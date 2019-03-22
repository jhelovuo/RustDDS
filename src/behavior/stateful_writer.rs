use crate::behavior::reader_proxy::ReaderProxy;
use crate::behavior::writer::{Writer, WriterAttributes};
use crate::structure::cache_change::CacheChange;
use crate::structure::change_kind::ChangeKind_t;
use crate::structure::data::Data;
use crate::structure::endpoint::{Endpoint, EndpointAttributes};
use crate::structure::entity::{Entity, EntityAttributes};
use crate::structure::guid::GUID_t;
use crate::structure::instance_handle::InstanceHandle_t;

pub struct StatefulWriter {
    /// The StatefulWriter keeps track of all the RTPS Readers matched with it.
    /// Each matched reader is represented by an instance of the ReaderProxy
    /// class.
    matched_readers: Vec<ReaderProxy>,
    entity: EntityAttributes,
    endpoint: EndpointAttributes,
    writer: WriterAttributes,
}

impl Entity for StatefulWriter {
    fn as_entity(&self) -> &EntityAttributes {
        &self.entity
    }
}

impl Endpoint for StatefulWriter {
    fn as_endpoint(&self) -> &EndpointAttributes {
        &self.endpoint
    }
}

impl Writer for StatefulWriter {
    fn as_writer(&self) -> &WriterAttributes {
        &self.writer
    }

    /// This operation creates a new CacheChange to be appended to the RTPS
    /// Writer’s HistoryCache. The sequence number of the CacheChange is
    /// automatically set to be the sequenceNumber of the previous change plus
    /// one.
    fn new_change(
        &mut self,
        kind: ChangeKind_t,
        data: Data,
        handle: InstanceHandle_t,
    ) -> CacheChange {
        unimplemented!();
    }
}

impl StatefulWriter {
    pub fn new() -> Self {
        unimplemented!();
    }

    pub fn matched_reader_add(&mut self, a_reader_proxy: ReaderProxy) {
        self.matched_readers.push(a_reader_proxy)
    }

    pub fn matched_reader_remove(&mut self, _a_reader_proxy: ReaderProxy) {
        unimplemented!();
    }

    pub fn matched_reader_lookup(
        &self,
        a_reader_guid: GUID_t,
    ) -> impl Iterator<Item = &ReaderProxy> {
        self.matched_readers
            .iter()
            .filter(move |proxy| proxy.remote_reader_guid == a_reader_guid)
    }

    pub fn is_acked_by_all(&self, _a_change: &CacheChange) {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

}
