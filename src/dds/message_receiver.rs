use crate::messages::protocol_version::ProtocolVersion;
use crate::messages::submessages::submessage::EntitySubmessage;
use crate::messages::vendor_id::VendorId;
use crate::structure::guid::{GuidPrefix, GUID};
use crate::dds::participant::DomainParticipant;
use crate::messages::submessages::submessage_header::SubmessageHeader;
use crate::messages::submessages::submessage_kind::SubmessageKind;
use crate::messages::submessages::submessage_flag::SubmessageFlag;
use crate::serialization::submessage::SubMessage;


use crate::structure::locator::{LocatorKind, LocatorList, Locator};

use crate::messages::header::Header;
use speedy::{Readable, Writable, Endianness};
use crate::structure::time::Time;

const RTPS_MESSAGE_HEADER_SIZE: usize = 20;

#[derive(Debug, PartialEq)]
pub struct MessageReceiver {
  participant_guid_prefix: GuidPrefix,

  pub source_version: ProtocolVersion,
  pub source_vendor_id: VendorId,
  pub source_guid_prefix: GuidPrefix,
  pub dest_guid_prefix: GuidPrefix,
  pub unicast_reply_locator_list: LocatorList,
  pub multicast_reply_locator_list: LocatorList,
  pub have_timestamp: bool,
  pub timestamp: Time,

  pos: usize,
}

impl MessageReceiver {
  pub fn new(participant_guid_prefix: GuidPrefix) -> MessageReceiver {
    // could be passed in as a parameter
    let locator_kind = LocatorKind::LOCATOR_KIND_UDPv4;

    MessageReceiver {
      participant_guid_prefix: participant_guid_prefix,

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
      
      pos: 0,
    }
  }

  pub fn reset(&mut self){
      self.source_version = ProtocolVersion::PROTOCOLVERSION;
      self.source_vendor_id = VendorId::VENDOR_UNKNOWN;
      self.source_guid_prefix = GuidPrefix::GUIDPREFIX_UNKNOWN;
      self.dest_guid_prefix = GuidPrefix::GUIDPREFIX_UNKNOWN;
      self.unicast_reply_locator_list.clear();
      self.multicast_reply_locator_list.clear();
      self.have_timestamp = false;
      self.timestamp = Time::TIME_INVALID;

      self.pos = 0;
  }

  pub fn handle_discovery_msg(&mut self, msg: Vec<u8>) {
    self.reset();
    self.dest_guid_prefix = self.participant_guid_prefix;

    if msg.len() < RTPS_MESSAGE_HEADER_SIZE {
      // Error, not even a Header
    }
    if !self.is_RTPS_header(&msg) {
      return; // vastaus?
    }

    let message_valid = true;
    let endianness = Endianness::LittleEndian;

    while self.pos < msg.len() {

      // Submessage header lenght is 4 bytes 
      // Handle errors in the deserialize function if needed
      let sm_header = SubMessage::deserialize_header(
        endianness,
        &msg[self.pos..self.pos+4]);
      self.pos += 4;
      
      let sm_length = sm_header.submessage_length as usize;
      let entity_sm = SubMessage::deserialize_msg(
        &sm_header,
        endianness,
        &msg[self.pos..(self.pos + sm_length)]);
      self.pos += sm_length;

      let submessage = SubMessage {
        header: sm_header,
        submessage: entity_sm.unwrap(),
      };

    }
  }

  pub fn handle_user_msg(&self, msg: Vec<u8>) {
    unimplemented!();
  }

  fn is_RTPS_header(&mut self, msg: &Vec<u8>) -> bool {
    // First 4 bytes 'R' 'T' 'P' 'S'
    if msg[0] != 0x52 || msg[1] != 0x54 || msg[2] != 0x50 || msg[3] != 0x53 {
      return false;
    } 
    self.pos += 4;

    // Check and set protocl version
    if msg[self.pos] > ProtocolVersion::PROTOCOLVERSION.major {
      // Error: Too new version
      return false;
    }
    self.source_version = ProtocolVersion {
      major: msg[self.pos],
      minor: msg[self.pos+1],
    };
    self.pos += 2;

    // Set vendor id
    self.source_vendor_id = VendorId::read_from_buffer(
      &msg[self.pos..self.pos+2]).unwrap();
    self.pos += 2;

    // Set source guid prefix
    self.source_guid_prefix = GuidPrefix::read_from_buffer(
      &msg[self.pos..self.pos+12]).unwrap();
    self.pos += 12;
    true
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

  #[test]
  fn mr_test_is_RTPS_header() {
    let test_header: Vec<u8> = vec![
      0x52, 0x54, 0x50, 0x53, 
      0x02, 0x03, 0x01, 0x0f,
      0x01, 0x0f, 0x99, 0x06, 
      0x78, 0x34, 0x00, 0x00, 
      0x01, 0x00, 0x00, 0x00, 
    ];
    let guid_new = GUID::new();
    let mut message_receiver = 
    MessageReceiver::new(guid_new.guidPrefix);
    assert!(message_receiver.is_RTPS_header(&test_header));
  }

}
