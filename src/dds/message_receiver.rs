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
  pos: usize,
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
      pos: 0,
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
      self.pos = 0;
  }

  fn is_RTPS_header(&mut self, msg: &Vec<u8>) -> bool {
    // First 4 bytes 'R' 'T' 'P' 'S'
    if msg[0] != 52u8 || msg[1] != 54u8 || msg[2] != 50u8 || msg[3] != 53u8 {
      // Error: Incorrect format
      return false;
    } 
    self.pos += 4;

    // Check and set protocl version
    if msg[self.pos] <= ProtocolVersion::PROTOCOLVERSION.major {
      self.receiver.source_version = ProtocolVersion {
        major: msg[self.pos],
        minor: msg[self.pos+1],
      };
      self.pos += 2;
    } else {
      // Error: Too new version
      return false;
    }
    // Set vendor id
    self.receiver.source_vendor_id = VendorId::read_from_buffer(
      &msg[self.pos..self.pos+2]).unwrap();
    self.pos += 2;

    // Set source guid prefix
    self.receiver.source_guid_prefix = GuidPrefix::read_from_buffer(
      &msg[self.pos..self.pos+12]).unwrap();
    true
  }



  pub fn handle_discovery_msg(&mut self, msg: Vec<u8>) {

    if msg.len() < RTPS_MESSAGE_HEADER_SIZE {
      // Error 
    }
    if !self.is_RTPS_header(&msg) {
      // Error
    }

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
