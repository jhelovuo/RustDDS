use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::submessages::submessage::EntitySubmessage;
use crate::messages::vendor_id::VendorId;
use crate::structure::guid::{GuidPrefix, GUID};

use crate::structure::locator::{LocatorKind, LocatorList, Locator};

use crate::messages::header::Header;
use speedy::{Readable, Writable, Endianness};
use crate::structure::time::Time;

const RTPS_MESSAGE_HEADER_SIZE: usize = 20;

#[derive(Debug, PartialEq)]
pub struct Receiver {
  pub source_version: ProtocolVersion,
  pub source_vendor_id: VendorId,
  pub source_guid_prefix: GuidPrefix,
  pub dest_guid_prefix: GuidPrefix,
  pub unicast_reply_locator_list: LocatorList,
  pub multicast_reply_locator_list: LocatorList,
  pub have_timestamp: bool,
  pub timestamp: Time,
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
  pub fn new() -> MessageReceiver {
    // could be passed in as a parameter
    let locator_kind = LocatorKind::LOCATOR_KIND_UDPv4;

    MessageReceiver {
      receiver: Receiver {
        source_version: ProtocolVersion::PROTOCOLVERSION,
        source_vendor_id: VendorId::VENDOR_UNKNOWN,
        source_guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
        dest_guid_prefix: GuidPrefix::GUIDPREFIX_UNKNOWN,
        unicast_reply_locator_list: vec![Locator {
          kind: locator_kind,
          address: Locator::LOCATOR_ADDRESS_INVALID,
          port: Locator::LOCATOR_PORT_INVALID,
        }],
        multicast_reply_locator_list: vec![Locator {
          kind: locator_kind,
          address: Locator::LOCATOR_ADDRESS_INVALID,
          port: Locator::LOCATOR_PORT_INVALID,
        }],
        have_timestamp: false,
        timestamp: Time::TIME_INVALID,
      },
      state: DeserializationState::ReadingHeader,
    }
  }

  pub fn reset(&mut self){
      self.receiver.source_version = ProtocolVersion::PROTOCOLVERSION;
      self.receiver.source_vendor_id = VendorId::VENDOR_UNKNOWN;
      self.receiver.source_guid_prefix = GuidPrefix::GUIDPREFIX_UNKNOWN;
      self.receiver.dest_guid_prefix = GuidPrefix::GUIDPREFIX_UNKNOWN;
      self.receiver.unicast_reply_locator_list.clear();
      self.receiver.multicast_reply_locator_list.clear();
      self.receiver.have_timestamp = false;
      self.receiver.timestamp = Time::TIME_INVALID;

      self.state = DeserializationState::ReadingHeader;
  }

  fn is_RTPS_header(&self, msg: &Vec<u8>) -> bool {
    if msg[0] != 'R' || msg[1] != 'T' || msg[2] != 'P' || msg[3] != 'S' {
      // Joku viesti?
      return false;
    } 
    true

  }

  pub fn handle_discovery_msg(&mut self, msg: Vec<u8>) {

    if msg.len() < RTPS_MESSAGE_HEADER_SIZE {
      // Error 
    }
    
    if !self.is_RTPS_header(&msg) {
      // Error
    }
    // First 4 bytes 'R' 'T' 'P' 'S'
    // next 2 protool version
    let protocol_version = ProtocolVersion { major: msg[4], minor: msg[5]};
    // next 2 vendor id
    let vendor_id = VendorId::read_from_buffer(&msg[6..8]).unwrap();
    // next 12 guid prefix
    let header = Header::read_from_buffer(&msg[8..20]).unwrap();


    self.state = DeserializationState::ReadingSubmessage;

    // Loop each subemssage
  }

      // All lenghts defined in RTPS spec
    // GuidPrefix 12 bytes
    // Entity id 4 bytes, first 3 entityKey, 4th entityKind
    // two MSB of entityKind tell if entity is build in, vendor or user defined
    // the information kind is encoded in the last 6 bits

    // create guidPrefix
    // from the guid prefix, the first two bytes identify vendor id
    // create entity id

    // with these create header and GUID

    // vendor id is of length 2 bytes
    // the protocol version is 2 bytes, major and minor
    // sequence number is of length 4 bytes high and 4 bytes low
    // sequence number set 8 + 4 + 4*M bytes
    // fragment number 4 bytes
    // fragment number set 4 + 4 + 4*M bytes
    // time stamp 4 + 4 bytes
    // locator list 4 + 4*M
    // where each M is 4 +4 + 16 bytes

  pub fn handle_user_msg(&self, msg: Vec<u8>) {
    unimplemented!();
  }

} // impl messageReceiver


#[cfg(test)]

mod tests{
  use super::*;

  #[test]
  fn mr_test_header() {

    let guid_new = GUID::new();

    let header = Header::new(guid_new.guidPrefix);
    
    let bytes = header.write_to_vec().unwrap();
    let new_header = Header::read_from_buffer(&bytes).unwrap();

    assert_eq!(header, new_header);
  }

}
