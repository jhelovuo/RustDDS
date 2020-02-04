use crate::common::validity_trait::Validity;
use crate::messages::ack_nack::AckNack;
use crate::messages::header::Header;
use crate::messages::protocol_version::ProtocolVersion_t;
use crate::messages::submessage::{EntitySubmessage, InterpreterSubmessage};
use crate::messages::submessage_flag::SubmessageFlag;
use crate::messages::submessage_header::SubmessageHeader;
use crate::messages::submessage_kind::SubmessageKind;
use crate::messages::vendor_id::VendorId_t;
use crate::structure::count::Count_t;
use crate::structure::entity_id::EntityId_t;
use crate::structure::guid_prefix::GuidPrefix_t;
use crate::structure::locator::{LocatorKind_t, LocatorList_t, Locator_t};
use crate::structure::sequence_number::SequenceNumber_t;
use crate::structure::sequence_number_set::SequenceNumberSet_t;
use crate::structure::time::Time_t;
use speedy::{Endianness, Readable, Writable};
use std::io::{Error, ErrorKind};

use bytes::BytesMut;
use tokio::codec::Decoder;

pub struct Receiver {
    pub source_version: ProtocolVersion_t,
    pub source_vendor_id: VendorId_t,
    pub source_guid_prefix: GuidPrefix_t,
    pub dest_guid_prefix: GuidPrefix_t,
    pub unicast_reply_locator_list: LocatorList_t,
    pub multicast_reply_locator_list: LocatorList_t,
    pub have_timestamp: bool,
    pub timestamp: Time_t,
}

enum DeserializationState {
    ReadingHeader,
    ReadingSubmessage,
    Finished,
}

pub struct MessageReceiver {
    receiver: Receiver,
    state: DeserializationState,
}

impl MessageReceiver {
    pub fn new(locator_kind: LocatorKind_t) -> Self {
        MessageReceiver {
            receiver: Receiver {
                source_version: ProtocolVersion_t::PROTOCOLVERSION,
                source_vendor_id: VendorId_t::VENDOR_UNKNOWN,
                source_guid_prefix: GuidPrefix_t::GUIDPREFIX_UNKNOWN,
                dest_guid_prefix: GuidPrefix_t::GUIDPREFIX_UNKNOWN,
                unicast_reply_locator_list: vec![Locator_t {
                    kind: locator_kind,
                    address: Locator_t::LOCATOR_ADDRESS_INVALID,
                    port: Locator_t::LOCATOR_PORT_INVALID,
                }],
                multicast_reply_locator_list: vec![Locator_t {
                    kind: locator_kind,
                    address: Locator_t::LOCATOR_ADDRESS_INVALID,
                    port: Locator_t::LOCATOR_PORT_INVALID,
                }],
                have_timestamp: false,
                timestamp: Time_t::TIME_INVALID,
            },
            state: DeserializationState::ReadingHeader,
        }
    }
    pub fn receiver(&self) -> &Receiver {
        &self.receiver
    }
}

impl Decoder for MessageReceiver {
    type Item = EntitySubmessage;
    type Error = std::io::Error;

    fn decode(&mut self, bytes: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::header::Header;

    macro_rules! message_decoding_test {
        (test_name = $name:ident, header = $header:expr,
        [$(submessage_header = $submessage_header:expr, submessage_entities = { $($entity:expr),* }),+]) => {
            mod $name {
                use super::*;

                fn convert_to_bytes() -> bytes::BytesMut {
                    let mut serialized_input: Vec<u8> = $header.write_to_vec(Endianness::NATIVE).unwrap();
                    $(
                        let mut submessage_header = $submessage_header;
                        let mut submessage_content: Vec<u8> = vec![];
                        $(
                            let serialized_submessage =
                                $entity.write_to_vec(submessage_header.flags.endianness_flag()).unwrap();
                            submessage_content.extend(serialized_submessage.into_iter());
                        )*

                        submessage_header.submessage_length = submessage_content.len() as u16;
                        submessage_header.write_to_vec(submessage_header.flags.endianness_flag()).unwrap();
                        serialized_input.extend(submessage_content.into_iter());
                    )+
                    bytes::BytesMut::from(serialized_input)
                }

                #[test]
                fn draft() {
                    let mut message_receiver = MessageReceiver::new(LocatorKind_t::LOCATOR_KIND_INVALID);
                    let mut buf = convert_to_bytes();
                    let result = message_receiver.decode(&mut buf);
                }
            }
        }
    }

    message_decoding_test!(
        test_name = single_ack_nack,
        header = Header::new(GuidPrefix_t::GUIDPREFIX_UNKNOWN),
        [
            submessage_header = SubmessageHeader {
                submessage_id: SubmessageKind::ACKNACK,
                flags: SubmessageFlag { flags: 0b00000000 },
                submessage_length: 24,
            },
            submessage_entities = {
                EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
                EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
                SequenceNumberSet_t::new(SequenceNumber_t::from(0)),
                Count_t::from(1)
            }
        ]
    );
}
