use common::locator;
use common::vendor_id;
use common::protocol_version;

struct Participant {
    protocol_version: protocol_version::ProtocolVersion_t,
    vendor_id: vendor_id::VendorId_t,
    default_unicast_locator_list: locator::Locator_t,
    default_multicast_locator_list: locator::Locator_t,
}
