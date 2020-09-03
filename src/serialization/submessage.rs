use std::convert::TryInto;
use enumflags2::BitFlags;

use crate::messages::submessages::submessage_header::SubmessageHeader;
use crate::messages::submessages::submessage::EntitySubmessage;
use crate::messages::submessages::submessages::*; 

use crate::messages::submessages::submessage_elements
  ::serialized_payload::{SerializedPayload};
use crate::messages::submessages::submessage_elements::parameter_list::ParameterList;

use crate::structure::guid::EntityId;
use crate::structure::sequence_number::SequenceNumber;

use crate::{messages::fragment_number::FragmentNumber, };
use speedy::{Context, Writer, Readable, Writable, Endianness};


#[derive(Debug, PartialEq)]
pub struct SubMessage {
  pub header: SubmessageHeader,
  pub submessage: Option<EntitySubmessage>,
  pub intepreterSubmessage: Option<InterpreterSubmessage>,
}

impl<'a> SubMessage
where
{
  
  fn deserialize_data_frag(
    buffer: &'a [u8],
    flags: BitFlags<DATAFRAG_Flags>
  ) -> Option<DataFrag> {
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
      representation_options: rep_options.try_into().expect("Unexpected array length in representation_options"),
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

  pub fn serialize_header(&self) -> Vec<u8> {
    let buffer = self.header.write_to_vec_with_ctx(Endianness::LittleEndian);
    buffer.unwrap()
  }

  pub fn serialize_msg(&self) -> Vec<u8> {
    let message: Vec<u8>;
    message = match &self.submessage {
      Some(EntitySubmessage::AckNack(msg, _)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      Some(EntitySubmessage::Data(msg, _)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      Some(EntitySubmessage::DataFrag(msg, _)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      Some(EntitySubmessage::Gap(msg,_)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      Some(EntitySubmessage::Heartbeat(msg, _)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      Some(EntitySubmessage::HeartbeatFrag(msg,_)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      Some(EntitySubmessage::NackFrag(msg,_)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      _ => {
        panic!();
      }
    };
    return message;
  }

  pub fn serialize_interpreter_msg(&self) -> Vec<u8> {
    let message: Vec<u8>;
    message = match &self.intepreterSubmessage {
      Some(InterpreterSubmessage::InfoDestination(msg,_)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      Some(InterpreterSubmessage::InfoReply(msg, _)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      Some(InterpreterSubmessage::InfoSource(msg,_)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      Some(InterpreterSubmessage::InfoTimestamp(msg, _)) => {
        msg.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap()
      }
      _ => {
        panic!();
      }
    };
    return message;
  }
}

impl<C: Context> Writable<C> for SubMessage {
  fn write_to<'a, T: ?Sized + Writer<C>>(&'a self, writer: &mut T) -> Result<(), C::Error> {
    let by = &self.serialize_header();
    let mesBy: Vec<u8>;
    if self.submessage.is_some() {
      mesBy = self.serialize_msg();
    } else {
      mesBy = self.serialize_interpreter_msg();
    }
    for b in by {
      writer.write_u8(*b)?;
    }
    for b in mesBy {
      writer.write_u8(b)?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use enumflags2::BitFlags;
  use super::SubMessage;
  use speedy::{Readable, Writable};
  use crate::{
    messages::submessages::submessages::*,
  };
  #[test]
  fn submessage_data_submessage_deserialization() {
    // this is wireshark captured shapesdemo dataSubmessage
    let serializedDataSubMessage: Vec<u8> = vec![
      0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x01,
      0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x00,
      0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x69, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0x1e,
      0x00, 0x00, 0x00,
    ];

    let header = 
      SubmessageHeader::read_from_buffer(&serializedDataSubMessage[0..4])
        .expect("could not create submessage header");
    println!("{:?}", header);

    let flags = BitFlags::<DATA_Flags>::from_bits_truncate(header.flags);
    let suba = Data::deserialize_data( &serializedDataSubMessage[4..] , flags )
      .expect("DATA deserialization failed.");
    let sub = SubMessage { header, submessage: Some(EntitySubmessage::Data(suba,flags))
        , intepreterSubmessage:None};

    println!("{:?}", sub);

    let mut messageBuffer = vec![];
    sub.write_to_buffer( &mut messageBuffer).expect("DATA serialization failed");

    assert_eq!(serializedDataSubMessage, messageBuffer);
  }
  /* TODO: rewrite rest of tests according to first one. Or better: use a common test function/macro
      to avoid copy/paste test code, since all of these start from reference byte vector, then deserialize,
      serialize again, and compare to reference.
  #[test]
  fn submessage_hearbeat_deserialization() {
    let serializedHeartbeatMessage: Vec<u8> = vec![
      0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00,
      0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x1f, 0x00,
      0x00, 0x00,
    ];
    let submessage_header = match SubMessage::deserialize_header(
      Endianness::LittleEndian,
      &serializedHeartbeatMessage[0..4],
    ) {
      Some(T) => T,
      None => {
        print!("could not create submessage header");
        return;
      } // rule 1. Could not create submessage header
    };
    println!("{:?}", submessage_header);

    let ha = Heartbeat::read_from_buffer_with_ctx(
      Endianness::LittleEndian,
      &serializedHeartbeatMessage[4..],
    )
    .unwrap();
    let mut messageBuffer = vec![];
    println!("heartbeatSubmessage : {:?}", ha);
    messageBuffer.append(
      &mut submessage_header
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap(),
    );
    messageBuffer.append(&mut ha.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap());
    assert_eq!(serializedHeartbeatMessage, messageBuffer);
  }
  #[test]
  fn submessage_info_dst_deserialization() {
    let serializedInfoDSTMessage: Vec<u8> = vec![
      0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 0x31, 0xa2, 0x28, 0x20, 0x02,
      0x08,
    ];
    let submessage_header = match SubMessage::deserialize_header(
      Endianness::LittleEndian,
      &serializedInfoDSTMessage[0..4],
    ) {
      Some(T) => T,
      None => {
        print!("could not create submessage header");
        return;
      } // rule 1. Could not create submessage header
    };
    println!("{:?}", submessage_header);

    let ha = InfoDestination::read_from_buffer_with_ctx(
      Endianness::LittleEndian,
      &serializedInfoDSTMessage[4..],
    )
    .unwrap();
    let mut messageBuffer = vec![];
    println!("info dst : {:?}", ha);
    messageBuffer.append(
      &mut submessage_header
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap(),
    );
    messageBuffer.append(&mut ha.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap());
    assert_eq!(serializedInfoDSTMessage, messageBuffer);
  }

  #[test]
  fn submessage_info_ts_deserialization() {
    let serializedInfoTSMessage: Vec<u8> = vec![
      0x09, 0x01, 0x08, 0x00, 0x1a, 0x15, 0xf3, 0x5e, 0x00, 0xcc, 0xfb, 0x13,
    ];
    let submessage_header = match SubMessage::deserialize_header(
      Endianness::LittleEndian,
      &serializedInfoTSMessage[0..4],
    ) {
      Some(T) => T,
      None => {
        print!("could not create submessage header");
        return;
      } // rule 1. Could not create submessage header
    };
    println!("{:?}", submessage_header);

    let ha = InfoTimestamp::read_from_buffer_with_ctx(
      Endianness::LittleEndian,
      &serializedInfoTSMessage[4..],
    )
    .unwrap();
    let mut messageBuffer = vec![];
    println!("timestamp : {:?}", ha);
    messageBuffer.append(
      &mut submessage_header
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap(),
    );
    messageBuffer.append(&mut ha.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap());
    assert_eq!(serializedInfoTSMessage, messageBuffer);
  }
  #[test]
  fn submessage_data_deserialization() {
    let serializedDatamessage: Vec<u8> = vec![
      0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x01,
      0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x00,
      0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x69, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0x1e,
      0x00, 0x00, 0x00,
    ];
    let submessage_header = match SubMessage::deserialize_header(
      Endianness::LittleEndian,
      &serializedDatamessage[0..4],
    ) {
      Some(T) => T,
      None => {
        print!("could not create submessage header");
        return;
      } // rule 1. Could not create submessage header
    };
    println!("{:?}", submessage_header);
    let helper = SubmessageFlagHelper::get_submessage_flags_helper_from_submessage_flag(
      &SubmessageKind::DATA,
      &submessage_header.flags,
    );
    let ha = Data::deserialize_data(
      &serializedDatamessage[4..].to_vec(),
      helper.get_endianness(),
      helper.InlineQosFlag,
      helper.DataFlag,
    );

    let mut messageBuffer = vec![];
    println!("datamessage  : {:?}", ha);
    messageBuffer.append(
      &mut submessage_header
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap(),
    );
    messageBuffer.append(&mut ha.write_to_vec_with_ctx(Endianness::LittleEndian).unwrap());
    assert_eq!(serializedDatamessage, messageBuffer);
  } */
}
