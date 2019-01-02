use crate::structure::locator;
use crate::structure::reliability_kind;
use crate::structure::topic_kind;

struct Endpoint {
    topic_kind: topic_kind::TopicKind_t,
    reliability_level: reliability_kind::ReliabilityKind_t,
    unicast_locator_list: locator::Locator_t,
    multicast_locator_list: locator::Locator_t,
}
