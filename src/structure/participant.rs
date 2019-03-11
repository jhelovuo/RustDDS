use crate::messages::protocol_version;
use crate::messages::vendor_id;
use crate::structure::locator;

/// Container of all RTPS entities that share common properties and are located
/// in a single address space.
struct Participant {
    /// Identifies the version of the RTPS protocol that the Participant uses to
    /// communicate.
    protocol_version: protocol_version::ProtocolVersion_t,

    /// Identifies the vendor of the RTPS middleware that contains the
    /// Participant.
    vendor_id: vendor_id::VendorId_t,

    /// Default list of unicast locators (transport, address, port combinations)
    /// that can be used to send messages to the Endpoints contained in the
    /// Participant.These are the unicast locators that will be used in case the
    /// Endpoint does not specify its own set of Locators.
    default_unicast_locator_list: locator::Locator_t,

    /// Default list of multicast locators (transport, address, port
    /// combinations) that can be used to send messages to the Endpoints
    /// contained in the Participant.These are the multicast locators that will
    /// be used in case the Endpoint does not specify its own set of Locators.
    default_multicast_locator_list: locator::Locator_t,
}
