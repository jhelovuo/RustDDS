use crate::messages::submessage_flag::SubmessageFlag;
use crate::messages::submessage_kind::SubmessageKind;
use speedy_derive::{Readable, Writable};

#[derive(Debug, PartialEq, Readable, Writable)]
pub struct SubmessageHeader {
    pub submessage_id: SubmessageKind,
    pub flags: SubmessageFlag,
    pub submessage_length: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = SubmessageHeader,
    {
        submessage_header_acknack,
        SubmessageHeader {
            submessage_id: SubmessageKind::ACKNACK,
            flags: SubmessageFlag { flags: 0x01 },
            submessage_length: 42,
        },
        le = [0x06, 0x01, 0x2A, 0x00],
        be = [0x06, 0x01, 0x00, 0x2A]
    },
    {
        submessage_header_gap,
        SubmessageHeader {
            submessage_id: SubmessageKind::GAP,
            flags: SubmessageFlag { flags: 0x03 },
            submessage_length: 7,
        },
        le = [0x08, 0x03, 0x07, 0x00],
        be = [0x08, 0x03, 0x00, 0x07]
    });
}
