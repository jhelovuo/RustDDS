use common::topic_kind;
use common::reliability_kind;
use common::locator;

struct Endpoint {
    topic_kind: topic_kind::TopicKind_t,
    reliability_level: reliability_kind::ReliabilityKind_t,
    unicast_locator_list: locator::Locator_t,
    multicast_locator_list: locator::Locator_t,
}
