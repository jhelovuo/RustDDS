extern crate rtps;

use self::rtps::message_receiver::{MessageReceiver};

#[test]
fn message_receiver_initialization() {
    let message_receiver = rtps::message_receiver::MessageReceiver::new(
        rtps::common::locator::LocatorKind_t::LOCATOR_KIND_UDPv4
    );

    assert_eq!(rtps::common::protocol_version::PROTOCOLVERSION,
               message_receiver.source_version);
    assert_eq!(rtps::common::vendor_id::VENDOR_UNKNOWN,
               message_receiver.source_vendor_id);
    assert_eq!(rtps::common::vendor_id::VENDOR_UNKNOWN,
               message_receiver.source_vendor_id);
    assert_eq!(rtps::common::guid_prefix::GUIDPREFIX_UNKNOWN,
               message_receiver.source_guid_prefix);
    assert_eq!(rtps::common::guid_prefix::GUIDPREFIX_UNKNOWN,
               message_receiver.dest_guid_prefix);
    assert_eq!(false, message_receiver.have_timestamp);
    assert_eq!(rtps::common::time::TIME_INVALID,
               message_receiver.timestamp);

    assert_eq!(1, message_receiver.unicast_reply_locator_list.len());
    assert_eq!(1, message_receiver.unicast_reply_locator_list.capacity());

    assert_eq!(1, message_receiver.multicast_reply_locator_list.len());
    assert_eq!(1, message_receiver.multicast_reply_locator_list.capacity());

    let locator_t = rtps::common::locator::Locator_t {
        kind: rtps::common::locator::LocatorKind_t::LOCATOR_KIND_UDPv4,
        address: rtps::common::locator::LOCATOR_ADDRESS_INVALID,
        port: rtps::common::locator::LOCATOR_PORT_INVALID
    };

    assert_eq!(Some(&locator_t),
               message_receiver.unicast_reply_locator_list.first());
    assert_eq!(Some(&locator_t),
               message_receiver.multicast_reply_locator_list.first());
}
