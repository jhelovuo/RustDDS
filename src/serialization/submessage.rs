use std::convert::TryInto;
use enumflags2::BitFlags;

use crate::messages::submessages::submessage_header::SubmessageHeader;
use crate::messages::submessages::submessage::EntitySubmessage;
use crate::messages::submessages::submessages::*;

use crate::messages::submessages::submessage_elements::serialized_payload::{SerializedPayload};
use crate::messages::submessages::submessage_elements::parameter_list::ParameterList;

use crate::structure::guid::EntityId;
use crate::structure::sequence_number::SequenceNumber;

use crate::{messages::fragment_number::FragmentNumber};
use speedy::{Context, Writer, Readable, Writable};

#[derive(Debug, PartialEq)]
pub struct SubMessage {
  pub header: SubmessageHeader,
  pub body: SubmessageBody,
}

#[derive(Debug, PartialEq)]
pub enum SubmessageBody {
  Entity(EntitySubmessage),
  Interpreter(InterpreterSubmessage),
}

impl<'a> SubMessage {
  fn deserialize_data_frag(buffer: &'a [u8], flags: BitFlags<DATAFRAG_Flags>) -> Option<DataFrag> {
    let mut pos: usize = 0;
    // TODO Check flag locations..?
    //let endianness_flag = msgheader.flags.is_flag_set(3);
    let inline_qoS_flag = flags.contains(DATAFRAG_Flags::InlineQos);
    //let non_standar_payload_flag = msgheader.flags.is_flag_set(1);

    pos += 2; // Ignore extra flags for now

    let octets_to_inline_qos = u16::read_from_buffer(&buffer[pos..pos + 2]).unwrap();
    pos += 2;

    let reader_id = EntityId::read_from_buffer(&buffer[pos..pos + 4]).unwrap();
    pos += 4;
    let writer_id = EntityId::read_from_buffer(&buffer[pos..pos + 4]).unwrap();
    pos += 4;
    let writer_sn = SequenceNumber::read_from_buffer(&buffer[pos..pos + 8]).unwrap();
    pos += 8;
    let fragment_starting_num = FragmentNumber::read_from_buffer(&buffer[pos..pos + 4]).unwrap();
    pos += 4;
    let fragments_in_submessage = u16::read_from_buffer(&buffer[pos..pos + 2]).unwrap();
    pos += 2;
    let fragment_size = u16::read_from_buffer(&buffer[pos..pos + 2]).unwrap();
    pos += 2;
    let data_size = u32::read_from_buffer(&buffer[pos..pos + 4]).unwrap();
    pos += 4;

    if pos != octets_to_inline_qos as usize + 4 {
      pos = octets_to_inline_qos as usize + 4; // Ignore
    }

    let mut inline_qos: Option<ParameterList> = None;
    if inline_qoS_flag {
      let inline_qos_size: usize = 4; // Is it this for sure??
      inline_qos =
        Some(ParameterList::read_from_buffer(&buffer[pos..pos + inline_qos_size]).unwrap());
      pos += inline_qos_size;
    }
    let rep_identifier_raw = u16::read_from_buffer(&buffer[pos..pos + 2]).unwrap();
    pos += 2;
    let rep_options = &buffer[pos..pos + 2];
    pos += 2;

    let payload_size: usize = buffer.len() - pos;

    let vec_value = Vec::read_from_buffer(&buffer[pos..pos + payload_size]).unwrap();

    let serialized_payload = SerializedPayload {
      representation_identifier: rep_identifier_raw,
      representation_options: rep_options
        .try_into()
        .expect("Unexpected array length in representation_options"),
      value: vec_value,
    };
    Some(DataFrag {
      reader_id,
      writer_id,
      writer_sn,
      fragment_starting_num,
      fragments_in_submessage,
      data_size,
      fragment_size,
      inline_qos,
      serialized_payload,
    })
  }
}

impl<C: Context> Writable<C> for SubMessage {
  fn write_to<'a, T: ?Sized + Writer<C>>(&'a self, writer: &mut T) -> Result<(), C::Error> {
    writer.write_value(&self.header)?;
    match &self.body {
      SubmessageBody::Entity(e) => writer.write_value(&e),
      SubmessageBody::Interpreter(i) => writer.write_value(&i),
    }
  }
}

#[cfg(test)]
mod tests {
  use enumflags2::BitFlags;
  use super::SubMessage;
  use speedy::{Readable, Writable};
  use crate::{messages::submessages::submessages::*};
  use crate::serialization::submessage::*;

  #[test]
  fn submessage_data_submessage_deserialization() {
    // this is wireshark captured shapesdemo dataSubmessage
    let serializedDataSubMessage: Vec<u8> = vec![
      0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x01,
      0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x00,
      0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x69, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0x1e,
      0x00, 0x00, 0x00,
    ];

    let header = SubmessageHeader::read_from_buffer(&serializedDataSubMessage[0..4])
      .expect("could not create submessage header");
    let flags = BitFlags::<DATA_Flags>::from_bits_truncate(header.flags);
    let suba = Data::deserialize_data(&serializedDataSubMessage[4..], flags)
      .expect("DATA deserialization failed.");
    let sub = SubMessage {
      header,
      body: SubmessageBody::Entity(EntitySubmessage::Data(suba, flags)),
    };
    println!("{:?}", sub);

    let messageBuffer = sub
      .write_to_vec()
      .expect("DATA serialization failed");

    assert_eq!(serializedDataSubMessage, messageBuffer);
  }

  #[test]
  fn submessage_hearbeat_deserialization() {
    let serializedHeartbeatMessage: Vec<u8> = vec![
      0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00,
      0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x1f, 0x00,
      0x00, 0x00,
    ];

    let header = SubmessageHeader::read_from_buffer(&serializedHeartbeatMessage[0..4])
      .expect("could not create submessage header");
    let flags = BitFlags::<HEARTBEAT_Flags>::from_bits_truncate(header.flags);
    let e = endianness_flag(header.flags);
    let suba = Heartbeat::read_from_buffer_with_ctx(e,&serializedHeartbeatMessage[4..])
      .expect("deserialization failed.");
    let sub = SubMessage {
      header,
      body: SubmessageBody::Entity(EntitySubmessage::Heartbeat(suba, flags)),
    };
    println!("{:?}", sub);

    let messageBuffer = sub
      .write_to_vec()
      .expect("serialization failed");

    assert_eq!(serializedHeartbeatMessage, messageBuffer);
  }
  
  #[test]
  fn submessage_info_dst_deserialization() {
    let serializedInfoDSTMessage: Vec<u8> = vec![
      0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 0x31, 0xa2, 0x28, 0x20, 0x02,
      0x08,
    ];

    let header = SubmessageHeader::read_from_buffer(&serializedInfoDSTMessage[0..4])
      .expect("could not create submessage header");
    let flags = BitFlags::<INFODESTINATION_Flags>::from_bits_truncate(header.flags);
    let e = endianness_flag(header.flags);
    let suba = InfoDestination::read_from_buffer_with_ctx(e,&serializedInfoDSTMessage[4..])
      .expect("deserialization failed.");
    let sub = SubMessage {
      header,
      body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoDestination(suba, flags)),
    };
    println!("{:?}", sub);

    let messageBuffer = sub
      .write_to_vec()
      .expect("serialization failed");

    assert_eq!(serializedInfoDSTMessage, messageBuffer);
  }
  
  #[test]
  fn submessage_info_ts_deserialization() {
    let serializedInfoTSMessage: Vec<u8> = vec![
      0x09, 0x01, 0x08, 0x00, 0x1a, 0x15, 0xf3, 0x5e, 0x00, 0xcc, 0xfb, 0x13,
    ];
    let header = SubmessageHeader::read_from_buffer(&serializedInfoTSMessage[0..4])
      .expect("could not create submessage header");
    let flags = BitFlags::<INFOTIMESTAMP_Flags>::from_bits_truncate(header.flags);
    let e = endianness_flag(header.flags);
    let suba = InfoTimestamp::read_from_buffer_with_ctx(e,&serializedInfoTSMessage[4..])
      .expect("deserialization failed.");
    let sub = SubMessage {
      header,
      body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoTimestamp(suba, flags)),
    };
    println!("{:?}", sub);

    let messageBuffer = sub
      .write_to_vec()
      .expect("serialization failed");

    assert_eq!(serializedInfoTSMessage, messageBuffer);

  }
  
}
