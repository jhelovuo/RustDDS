use crate::messages::submessages::submessage_header::SubmessageHeader;
use crate::messages::submessages::submessage::EntitySubmessage;
use crate::messages::submessages::submessages::*;

use speedy::{Context, Writer, Writable};

#[derive(Debug, PartialEq, Clone)]
pub struct SubMessage {
  pub header: SubmessageHeader,
  pub body: SubmessageBody,
}

// TODO: Submessages should implement some Length trait that returns the length of
// Submessage in bytes. This is needed for Submessage construction.
#[derive(Debug, PartialEq, Clone)]
pub enum SubmessageBody {
  Entity(EntitySubmessage),
  Interpreter(InterpreterSubmessage),
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
  use bytes::Bytes;
    use enumflags2::BitFlags;
  use log::info;
  use super::SubMessage;
  use speedy::{Readable, Writable};
  use crate::{messages::submessages::submessages::*};
  use crate::serialization::submessage::*;

  #[test]
  fn submessage_data_submessage_deserialization() {
    // this is wireshark captured shapesdemo dataSubmessage
    let serializedDataSubMessage = Bytes::from_static(&[
      0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x01,
      0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x00,
      0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x69, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0x1e,
      0x00, 0x00, 0x00,
    ]);

    let header = SubmessageHeader::read_from_buffer(&serializedDataSubMessage[0..4])
      .expect("could not create submessage header");
    let flags = BitFlags::<DATA_Flags>::from_bits_truncate(header.flags);
    let suba = Data::deserialize_data(serializedDataSubMessage.slice(4..), flags)
      .expect("DATA deserialization failed.");
    let sub = SubMessage {
      header,
      body: SubmessageBody::Entity(EntitySubmessage::Data(suba, flags)),
    };
    info!("{:?}", sub);

    let messageBuffer = sub.write_to_vec().expect("DATA serialization failed");

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
    let suba = Heartbeat::read_from_buffer_with_ctx(e, &serializedHeartbeatMessage[4..])
      .expect("deserialization failed.");
    let sub = SubMessage {
      header,
      body: SubmessageBody::Entity(EntitySubmessage::Heartbeat(suba, flags)),
    };
    info!("{:?}", sub);

    let messageBuffer = sub.write_to_vec().expect("serialization failed");

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
    let suba = InfoDestination::read_from_buffer_with_ctx(e, &serializedInfoDSTMessage[4..])
      .expect("deserialization failed.");
    let sub = SubMessage {
      header,
      body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoDestination(suba, flags)),
    };
    info!("{:?}", sub);

    let messageBuffer = sub.write_to_vec().expect("serialization failed");

    assert_eq!(serializedInfoDSTMessage, messageBuffer);
  }

  // #[test]
  // fn submessage_info_ts_deserialization() {
  //   let serializedInfoTSMessage: Vec<u8> = vec![
  //     0x09, 0x01, 0x08, 0x00, 0x1a, 0x15, 0xf3, 0x5e, 0x00, 0xcc, 0xfb, 0x13,
  //   ];
  //   let header = SubmessageHeader::read_from_buffer(&serializedInfoTSMessage[0..4])
  //     .expect("could not create submessage header");
  //   let flags = BitFlags::<INFOTIMESTAMP_Flags>::from_bits_truncate(header.flags);
  //   let e = endianness_flag(header.flags);
  //   let suba = InfoTimestamp::read_from_buffer_with_ctx(e, &serializedInfoTSMessage[4..])
  //     .expect("deserialization failed.");
  //   let sub = SubMessage {
  //     header,
  //     body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoTimestamp(suba, flags)),
  //   };
  //   info!("{:?}", sub);

  //   let messageBuffer = sub.write_to_vec().expect("serialization failed");

  //   assert_eq!(serializedInfoTSMessage, messageBuffer);
  // }
}
