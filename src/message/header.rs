use common::protocol_id;
use common::protocol_version;
use common::vendor_id;
use common::guid_prefix;

enum SubmessageKind {
    PAD = 0x01,
    ACKNACK = 0x06,
    HEARTBEAT = 0x07,
    GAP = 0x08,
    INFO_TS = 0x09,
    INFO_SRC = 0x0c,
    INFO_REPLAY_IP4 = 0x0d,
    INFO_DST = 0x0e,
    INFO_REPLAY = 0x0f,
    NACK_FRAG = 0x12,
    HEARTBEAT_FRAG = 0x13,
    DATA = 0x15,
    DATA_FRAG = 0x16
}

struct Header {
    protocol_id: protocol_id::ProtocolId_t,
    protocol_version: protocol_version::ProtocolVersion_t,
    vendor_id: vendor_id::VendorId_t,
    guid_prefix: guid_prefix::GuidPrefix_t,
}

struct SubmessageHeader {
    
}
