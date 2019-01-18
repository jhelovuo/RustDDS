use crate::behavior::reader_locator::ReaderLocator;
use crate::structure::locator::Locator_t;
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
}

impl StatelessWriter {
    pub fn new(resend_data_period: Time_t) -> Self {
        StatelessWriter {
            resend_data_period: resend_data_period,
            reader_locators: vec![],
        }
    }

    pub fn reader_locator_add(&mut self, a_locator: ReaderLocator) {
        unimplemented!();
    }

    pub fn reader_locator_remove(&mut self, a_locator: ReaderLocator) {
        unimplemented!();
    }

    pub fn unsent_changes_reset(&mut self) {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

}
