extern crate rtps;

use self::rtps::common::protocol_id::{ProtocolId_t, PROTOCOL_RTPS};
use crate::common::protocol_id_test::rtps::message::validity_trait::Validity;

assert_ser_de!({
    protocol_rtps,
    PROTOCOL_RTPS,
    le = [0x52, 0x54, 0x50, 0x53],
    be = [0x52, 0x54, 0x50, 0x53]
});

#[test]
fn validity() {
    let protocol_id = PROTOCOL_RTPS;
    assert!(protocol_id.valid());
    let protocol_id = ProtocolId_t { protocol_id: ['S','P','T','R'] };
    assert!(!protocol_id.valid());
}
