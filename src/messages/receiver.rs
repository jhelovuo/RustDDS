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

#[derive(Debug, PartialEq)]
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

impl Default for Receiver {
    fn default() -> Self {
        Receiver {
            source_version: ProtocolVersion_t::PROTOCOLVERSION,
            source_vendor_id: VendorId_t::VENDOR_UNKNOWN,
            source_guid_prefix: GuidPrefix_t::GUIDPREFIX_UNKNOWN,
            dest_guid_prefix: GuidPrefix_t::GUIDPREFIX_UNKNOWN,
            unicast_reply_locator_list: vec![Locator_t {
                kind: LocatorKind_t::LOCATOR_KIND_INVALID,
                address: Locator_t::LOCATOR_ADDRESS_INVALID,
                port: Locator_t::LOCATOR_PORT_INVALID,
            }],
            multicast_reply_locator_list: vec![Locator_t {
                kind: LocatorKind_t::LOCATOR_KIND_INVALID,
                address: Locator_t::LOCATOR_ADDRESS_INVALID,
                port: Locator_t::LOCATOR_PORT_INVALID,
            }],
            have_timestamp: false,
            timestamp: Time_t::TIME_INVALID,
        }
    }
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

    struct EntitySubmessageIterator {
        message_receiver: MessageReceiver,
        bytes: bytes::BytesMut,
    }

    impl Iterator for EntitySubmessageIterator {
        type Item = Result<Option<EntitySubmessage>, std::io::Error>;

        fn next(&mut self) -> Option<Self::Item> {
            Some(self.message_receiver.decode(&mut self.bytes))
        }
    }

    macro_rules! message_decoding_test {
        (test_name = $name:ident, header = $header:expr,
        [$(submessage_header = $submessage_header:expr, submessage_entities = { $($entity:expr),* }),+],
        expected_notifications = [ $($expected_notification:expr),* ],
        receiver_state = $receiver_state:expr) => {
            mod $name {
                use super::*;

                fn serialize_into_bytes() -> bytes::BytesMut {
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

                #[ignore]
                #[test]
                fn test_submessage_decoding() {
                    let messages_iterator = EntitySubmessageIterator {
                        message_receiver: MessageReceiver::new(LocatorKind_t::LOCATOR_KIND_INVALID),
                        bytes: serialize_into_bytes()
                    };
                    let expected_notifications = vec![$($expected_notification),*]
                        .into_iter()
                        .inspect(|expectation| {
                            println!("Expected notification: {:#?}", expectation)
                        });

                    let decoder_output = messages_iterator
                        .take(10*expected_notifications.len())
                        .filter(|maybe_message| match maybe_message {
                            Ok(None) => false,
                            _ => true
                        })
                        .map(|maybe_parsed_message|
                            match maybe_parsed_message {
                                Ok(Some(parsed_message)) => parsed_message,
                                _ => unreachable!()
                            }
                        )
                        .inspect(|parsed_message| {
                            println!("Parsed message: {:#?}", parsed_message)
                        });

                    assert!(decoder_output.eq(expected_notifications));
                    // assert_eq!(&$receiver_state, messages_iterator.message_receiver.receiver());
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
                flags: SubmessageFlag { flags: 0b0000_0000 },
                submessage_length: 24,
            },
            submessage_entities = {
                EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
                EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
                SequenceNumberSet_t::new(SequenceNumber_t::from(0)),
                Count_t::from(1)
            }
        ],
        expected_notifications = [
            EntitySubmessage::AckNack(
                AckNack {
                    reader_id: EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_READER,
                    writer_id: EntityId_t::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_WRITER,
                    reader_sn_state: SequenceNumberSet_t::new(SequenceNumber_t::from(0)),
                    count: Count_t::from(1)
                },
                SubmessageFlag { flags: 0b0000_0000 }
            )],
        receiver_state = Receiver::default()
    );
}
