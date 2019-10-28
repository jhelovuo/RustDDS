use crate::behavior::reader_locator::ReaderLocator;
use crate::behavior::writer::{Writer, WriterAttributes};
use crate::structure::cache_change::CacheChange;
use crate::structure::change_kind::ChangeKind_t;
use crate::structure::data::Data;
use crate::structure::endpoint::{Endpoint, EndpointAttributes};
use crate::structure::entity::{Entity, EntityAttributes};
use crate::structure::instance_handle::InstanceHandle_t;
use crate::structure::sequence_number::SequenceNumber_t;
use crate::structure::time::Time_t;

/// Specialization of RTPS Writer used for the Stateless Reference
/// Implementation. The RTPS StatelessWriter has no knowledge of the number of
/// matched readers, nor does it maintain any state for each matched RTPS Reader
/// endpoint. The RTPS StatelessWriter maintains only the RTPS Locator_t list
/// that should be used to send information to the matched readers
struct StatelessWriter {
    /// Protocol tuning parameter that indicates that the StatelessWriter
    /// re-sends all the changes in the writer’s HistoryCache to
    /// all the Locators periodically each resendPeriod
    resend_data_period: Time_t,

    /// The StatelessWriter maintains the list of locators
    /// to which it sends the CacheChanges. This list may include
    /// both unicast and multicast locator
    reader_locators: Vec<ReaderLocator>,

    entity_attributes: EntityAttributes,
    endpoint_attributes: EndpointAttributes,
    writer_attributes: WriterAttributes,
}

impl Entity for StatelessWriter {
    fn as_entity(&self) -> &EntityAttributes {
        &self.entity_attributes
    }
}

impl Endpoint for StatelessWriter {
    fn as_endpoint(&self) -> &EndpointAttributes {
        &self.endpoint_attributes
    }
}

impl Writer for StatelessWriter {
    fn as_writer(&self) -> &WriterAttributes {
        &self.writer_attributes
    }

    fn new_change(
        &mut self,
        kind: ChangeKind_t,
        data: Data,
        handle: InstanceHandle_t,
    ) -> CacheChange {
        self.writer_attributes.last_change_sequence_number =
            self.writer_attributes.last_change_sequence_number + SequenceNumber_t::from(1);

        CacheChange {
            kind: kind,
            writer_guid: self.entity_attributes.guid,
            data_value: data,
            instance_handle: handle,
            sequence_number: self.writer_attributes.last_change_sequence_number,
        }
    }
}

impl StatelessWriter {
    pub fn new(
        entity_attributes: EntityAttributes,
        endpoint_attributes: EndpointAttributes,
        writer_attributes: WriterAttributes,
        resend_data_period: Time_t,
    ) -> Self {
        StatelessWriter {
            entity_attributes: entity_attributes,
            endpoint_attributes: endpoint_attributes,
            writer_attributes: writer_attributes,
            resend_data_period: resend_data_period,
            reader_locators: vec![],
        }
    }

    pub fn reader_locator_add(&mut self, a_locator: ReaderLocator) {
        self.reader_locators.push(a_locator)
    }

    pub fn reader_locator_remove(&mut self, a_locator: ReaderLocator) {
        self.reader_locators.retain(|x| x != &a_locator)
    }

    pub fn unsent_changes_reset(&mut self) {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
