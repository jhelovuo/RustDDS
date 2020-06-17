use crate::messages::header::Header;
use crate::types::submessage::SubMessage;

use speedy::{Readable, Writable, Endianness};

#[derive(Debug)]
pub struct Message {
  header: Header,
  submessages: Vec<SubMessage>,
}

impl<'a> Message {
  pub fn deserialize_header(context: Endianness, buffer: &'a [u8]) -> Header {
    Header::read_from_buffer_with_ctx(context, buffer).unwrap()
  }

  pub fn serialize_header(self) -> Vec<u8> {
    let buffer = self.header.write_to_vec_with_ctx(Endianness::LittleEndian);
    buffer.unwrap()
  }

  pub fn addSubmessage(mut self, submessage: SubMessage) {
    self.submessages.push(submessage);
  }

  pub fn removeSubmessage(mut self, index: usize) {
    self.submessages.remove(index);
  }

  pub fn submessages(self) -> Vec<SubMessage> {
    self.submessages
  }
}
