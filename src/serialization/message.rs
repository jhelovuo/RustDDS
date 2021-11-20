use std::{collections::BTreeSet, io};

#[allow(unused_imports)]
use log::{debug, error, trace, warn};
use speedy::{Context, Endianness, Readable, Writable, Writer};
use enumflags2::BitFlags;
use bytes::Bytes;

use crate::{
  dds::{
    data_types::{DDSTimestamp, EntityId, GUID},
    ddsdata::DDSData,
    writer::Writer as RtpsWriter,
  },
  messages::{
    header::Header,
    protocol_id::ProtocolId,
    protocol_version::ProtocolVersion,
    submessages::{
      submessage::EntitySubmessage,
      submessage_elements::{
        parameter::Parameter, parameter_list::ParameterList,
        serialized_payload::RepresentationIdentifier,
      },
      submessages::{SubmessageKind, *},
    },
    vendor_id::VendorId,
  },
  serialization::submessage::{SubMessage, SubmessageBody},
  structure::{
    cache_change::CacheChange,
    entity::RTPSEntity,
    guid::{EntityKind, GuidPrefix},
    parameter_id::ParameterId,
    sequence_number::{SequenceNumber, SequenceNumberSet},
    time::Timestamp,
  },
};

#[derive(Debug, Clone)]
pub(crate) struct Message {
  pub header: Header,
  pub submessages: Vec<SubMessage>,
}

impl<'a> Message {
  // pub fn deserialize_header(context: Endianness, buffer: &'a [u8]) -> Header {
  //   Header::read_from_buffer_with_ctx(context, buffer).unwrap()
  // }

  // pub fn serialize_header(self) -> Vec<u8> {
  //   let buffer = self.header.write_to_vec_with_ctx(Endianness::LittleEndian);
  //   buffer.unwrap()
  // }

  pub fn add_submessage(&mut self, submessage: SubMessage) {
    self.submessages.push(submessage);
  }

  // pub fn remove_submessage(mut self, index: usize) {
  //   self.submessages.remove(index);
  // }

  #[cfg(test)]
  pub fn submessages(self) -> Vec<SubMessage> {
    self.submessages
  }

  #[cfg(test)]
  pub fn set_header(&mut self, header: Header) {
    self.header = header;
  }

  // pub fn get_data_sub_message_sequence_numbers(&self) ->
  // HashSet<SequenceNumber> {   let mut sequence_numbers = HashSet::new();
  //   for mes in self.submessages.iter() {
  //     if let SubmessageBody::Entity(EntitySubmessage::Data(data_subm, _)) =
  // &mes.body {       sequence_numbers.insert(data_subm.writer_sn);
  //     }
  //   }
  //   sequence_numbers
  // }

  // We implement this instead of Speedy trait Readable, because
  // we need to run-time decide which endianness we input. Speedy requires the
  // top level to fix that. And there seems to be no reasonable way to change
  // endianness. TODO: The error type should be something better
  pub fn read_from_buffer(buffer: Bytes) -> io::Result<Message> {
    // The Header deserializes the same
    let rtps_header =
      Header::read_from_buffer(&buffer).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut message = Message::new(rtps_header);
    let mut submessages_left: Bytes = buffer.slice(20..); // header is 20 bytes
                                                          // submessage loop
    while !submessages_left.is_empty() {
      let sub_header = SubmessageHeader::read_from_buffer(&submessages_left)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
      // Try to figure out how large this submessage is.
      let sub_header_length = 4; // 4 bytes
      let sub_content_length = if sub_header.content_length == 0 {
        // RTPS spec 2.3, section 9.4.5.1.3:
        //           In case octetsToNextHeader==0 and the kind of Submessage is
        // NOT PAD or INFO_TS, the Submessage is the last Submessage in the Message
        // and extends up to the end of the Message. This makes it possible to send
        // Submessages larger than 64k (the size that can be stored in the
        // octetsToNextHeader field), provided they are the last Submessage in the
        // Message. In case the octetsToNextHeader==0 and the kind of Submessage is
        // PAD or INFO_TS, the next Submessage header starts immediately after the
        // current Submessage header OR the PAD or INFO_TS is the last Submessage
        // in the Message.
        match sub_header.kind {
          SubmessageKind::PAD | SubmessageKind::INFO_TS => 0,
          _not_pad_or_info_ts => submessages_left.len() - sub_header_length,
        }
      } else {
        sub_header.content_length as usize
      };

      // we have to use temporary variable new_submessages_left to avoid creating
      // another submessages_left
      // let (sub_buffer, new_submessages_left) =
      //   submessages_left.split_at(sub_header_length + sub_content_length);
      // submessages_left = new_submessages_left;
      // split fisrt buters to new buffer
      let mut sub_buffer = submessages_left.split_to(sub_header_length + sub_content_length);
      // split tail part (content) to new buffer
      let sub_content_buffer = sub_buffer.split_off(sub_header_length);

      let e = endianness_flag(sub_header.flags);
      let mk_e_subm = move |s: EntitySubmessage| {
        Ok(SubMessage {
          header: sub_header,
          body: SubmessageBody::Entity(s),
        })
      };
      let mk_i_subm = move |s: InterpreterSubmessage| {
        Ok(SubMessage {
          header: sub_header,
          body: SubmessageBody::Interpreter(s),
        })
      };

      let new_submessage_result: io::Result<SubMessage> = match sub_header.kind {
        SubmessageKind::DATA => {
          // Manually implemented deserialization for DATA. Speedy does not quite cut it.
          let f = BitFlags::<DATA_Flags>::from_bits_truncate(sub_header.flags);
          mk_e_subm(EntitySubmessage::Data(
            Data::deserialize_data(sub_content_buffer, f)?,
            f,
          ))
        }

        SubmessageKind::DATA_FRAG => {
          // Manually implemented deserialization for DATA. Speedy does not quite cut it.
          let f = BitFlags::<DATAFRAG_Flags>::from_bits_truncate(sub_header.flags);
          mk_e_subm(EntitySubmessage::DataFrag(
            DataFrag::deserialize(sub_content_buffer, f)?,
            f,
          ))
        }

        SubmessageKind::GAP => {
          let f = BitFlags::<GAP_Flags>::from_bits_truncate(sub_header.flags);
          mk_e_subm(EntitySubmessage::Gap(
            Gap::read_from_buffer_with_ctx(e, &sub_content_buffer)?,
            f,
          ))
        }

        SubmessageKind::ACKNACK => {
          let f = BitFlags::<ACKNACK_Flags>::from_bits_truncate(sub_header.flags);
          mk_e_subm(EntitySubmessage::AckNack(
            AckNack::read_from_buffer_with_ctx(e, &sub_content_buffer)?,
            f,
          ))
        }

        SubmessageKind::NACK_FRAG => {
          let f = BitFlags::<NACKFRAG_Flags>::from_bits_truncate(sub_header.flags);
          mk_e_subm(EntitySubmessage::NackFrag(
            NackFrag::read_from_buffer_with_ctx(e, &sub_content_buffer)?,
            f,
          ))
        }

        SubmessageKind::HEARTBEAT => {
          let f = BitFlags::<HEARTBEAT_Flags>::from_bits_truncate(sub_header.flags);
          mk_e_subm(EntitySubmessage::Heartbeat(
            Heartbeat::read_from_buffer_with_ctx(e, &sub_content_buffer)?,
            f,
          ))
        }

        // interpreter submessages
        SubmessageKind::INFO_DST => {
          let f = BitFlags::<INFODESTINATION_Flags>::from_bits_truncate(sub_header.flags);
          mk_i_subm(InterpreterSubmessage::InfoDestination(
            InfoDestination::read_from_buffer_with_ctx(e, &sub_content_buffer)?,
            f,
          ))
        }
        SubmessageKind::INFO_SRC => {
          let f = BitFlags::<INFOSOURCE_Flags>::from_bits_truncate(sub_header.flags);
          mk_i_subm(InterpreterSubmessage::InfoSource(
            InfoSource::read_from_buffer_with_ctx(e, &sub_content_buffer)?,
            f,
          ))
        }
        SubmessageKind::INFO_TS => {
          let f = BitFlags::<INFOTIMESTAMP_Flags>::from_bits_truncate(sub_header.flags);
          let tso = if f.contains(INFOTIMESTAMP_Flags::Invalidate) {
            None
          } else {
            Some(Timestamp::read_from_buffer_with_ctx(
              e,
              &sub_content_buffer,
            )?)
          };
          mk_i_subm(InterpreterSubmessage::InfoTimestamp(
            InfoTimestamp { timestamp: tso },
            f,
          ))
        }
        SubmessageKind::INFO_REPLY => {
          let f = BitFlags::<INFOREPLY_Flags>::from_bits_truncate(sub_header.flags);
          mk_i_subm(InterpreterSubmessage::InfoReply(
            InfoReply::read_from_buffer_with_ctx(e, &sub_content_buffer)?,
            f,
          ))
        }
        SubmessageKind::PAD => {
          continue; // nothing to do here
        }
        unknown_kind => {
          let kind = u8::from(unknown_kind);
          if kind >= 0x80 {
            // Kinds 0x80 - 0xFF are vendor-specific.
            trace!(
              "Received vendor-specific submessage kind {:?}",
              unknown_kind
            );
            trace!("Submessage was {:?}", &sub_buffer);
          } else {
            // Kind is 0x00 - 0x7F, is should be in the standard.
            error!("Received unknown submessage kind {:?}", unknown_kind);
            debug!("Submessage was {:?}", &sub_buffer);
          }
          continue;
        }
      }; // match

      message.submessages.push(new_submessage_result?)
    } // loop

    Ok(message)
  }
}

impl Message {
  pub fn new(header: Header) -> Message {
    Message {
      header,
      submessages: vec![],
    }
  }
}

impl Default for Message {
  fn default() -> Self {
    Message {
      header: Header::new(GuidPrefix::UNKNOWN),
      submessages: vec![],
    }
  }
}

impl<C: Context> Writable<C> for Message {
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    writer.write_value(&self.header)?;
    for x in &self.submessages {
      writer.write_value(&x)?;
    }
    Ok(())
  }
}

pub(crate) struct MessageBuilder {
  submessages: Vec<SubMessage>,
}

impl MessageBuilder {
  pub fn new() -> MessageBuilder {
    MessageBuilder {
      submessages: Vec::new(),
    }
  }

  pub fn dst_submessage(
    mut self,
    endianness: Endianness,
    guid_prefix: GuidPrefix,
  ) -> MessageBuilder {
    let flags = BitFlags::<INFODESTINATION_Flags>::from_endianness(endianness);
    let submessage_header = SubmessageHeader {
      kind: SubmessageKind::INFO_DST,
      flags: flags.bits(),
      content_length: 12u16,
      //InfoDST length is always 12 because message contains only GuidPrefix
    };
    let dst_submessage = SubMessage {
      header: submessage_header,
      body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoDestination(
        InfoDestination { guid_prefix },
        flags,
      )),
    };

    self.submessages.push(dst_submessage);
    self
  }

  /// Argument Some(timestamp) means that a timestamp is sent.
  /// Argument None means "invalidate", i.e. the previously sent InfoTimestamp
  /// submessage no longer applies.
  pub fn ts_msg(
    mut self,
    endianness: Endianness,
    timestamp: Option<DDSTimestamp>,
  ) -> MessageBuilder {
    let mut flags = BitFlags::<INFOTIMESTAMP_Flags>::from_endianness(endianness);
    if timestamp.is_none() {
      flags |= INFOTIMESTAMP_Flags::Invalidate;
    }

    let content_length = match timestamp {
      Some(_) => 8, // Timestamp is serialized as 2 x 32b words
      None => 0,    // Not serialized at all
    };

    let submessage_header = SubmessageHeader {
      kind: SubmessageKind::INFO_TS,
      flags: flags.bits(),
      content_length,
    };

    let submsg = SubMessage {
      header: submessage_header,
      body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoTimestamp(
        InfoTimestamp { timestamp },
        flags,
      )),
    };

    self.submessages.push(submsg);
    self
  }

  pub fn data_msg(
    mut self,
    cache_change: CacheChange,
    reader_entity_id: EntityId,
    writer_entity_id: EntityId,
    endianness: Endianness,
  ) -> MessageBuilder {
    let inline_qos = match cache_change.data_value {
      DDSData::Data {..} /*| DDSData::DataFrags{..}*/  => None,
      DDSData::DisposeByKey {..} => None,
      DDSData::DisposeByKeyHash{ key_hash, .. } => {
        let mut param_list = ParameterList::new();
        let key_hash_param = Parameter {
          parameter_id: ParameterId::PID_KEY_HASH,
          value: key_hash.to_vec(),
        };
        param_list.parameters.push(key_hash_param);
        let status_info = Parameter::create_pid_status_info_parameter(true, true, false);
        param_list.parameters.push(status_info);
        Some(param_list)
      }
    };

    let mut data_message = Data {
      reader_id: reader_entity_id,
      writer_id: writer_entity_id,
      writer_sn: cache_change.sequence_number,
      inline_qos,
      serialized_payload: match cache_change.data_value {
        DDSData::Data {
          ref serialized_payload,
        } => Some(serialized_payload.clone()), // contents is Bytes
        DDSData::DisposeByKey { ref key, .. } => Some(key.clone()),
        _ => None,
      },
    };

    // TODO: please explain this logic here:
    if writer_entity_id.kind() == EntityKind::WRITER_WITH_KEY_BUILT_IN {
      if let Some(sp) = data_message.serialized_payload.as_mut() {
        sp.representation_identifier = RepresentationIdentifier::PL_CDR_LE
      }
    }

    let flags: BitFlags<DATA_Flags> = BitFlags::<DATA_Flags>::from_endianness(endianness)
      | (match cache_change.data_value {
        DDSData::Data { .. } => BitFlags::<DATA_Flags>::from_flag(DATA_Flags::Data),
        DDSData::DisposeByKey { .. } => BitFlags::<DATA_Flags>::from_flag(DATA_Flags::Key),
        DDSData::DisposeByKeyHash { .. } => {
          BitFlags::<DATA_Flags>::from_flag(DATA_Flags::InlineQos)
        }
      });
    // TODO: This is stupid. There should be an easier way to get the submessage
    // length than serializing it!
    let size = data_message
      .write_to_vec_with_ctx(endianness)
      .unwrap()
      .len() as u16;

    self.submessages.push(SubMessage {
      header: SubmessageHeader {
        kind: SubmessageKind::DATA,
        flags: flags.bits(),
        content_length: size,
      },
      body: SubmessageBody::Entity(EntitySubmessage::Data(data_message, flags)),
    });
    self
  }

  // TODO: We should optimize this entire thing to allow long contiguous
  // irrelevant set to be represented as start_sn +
  pub fn gap_msg(
    mut self,
    irrelevant_sns: BTreeSet<SequenceNumber>,
    writer: &RtpsWriter,
    reader_guid: GUID,
  ) -> MessageBuilder {
    match (
      irrelevant_sns.iter().next(),
      irrelevant_sns.iter().next_back(),
    ) {
      (Some(&base), Some(&_top)) => {
        let gap_list = SequenceNumberSet::from_base_and_set(base, &irrelevant_sns);
        let gap = Gap {
          reader_id: reader_guid.entity_id,
          writer_id: writer.entity_id(),
          gap_start: base,
          gap_list,
        };
        let gap_flags = BitFlags::<GAP_Flags>::from_endianness(writer.endianness);
        gap
          .create_submessage(gap_flags)
          .map(|s| self.submessages.push(s));
      }
      (_, _) => error!("gap_msg called with empty SN set. Skipping GAP submessage"),
    }
    self
  }

  pub fn heartbeat_msg(
    mut self,
    writer: &RtpsWriter,
    reader_entityid: EntityId,
    set_final_flag: bool,
    set_liveliness_flag: bool,
  ) -> MessageBuilder {
    let first = writer.first_change_sequence_number;
    let last = writer.last_change_sequence_number;

    let heartbeat = Heartbeat {
      reader_id: reader_entityid,
      writer_id: writer.entity_id(),
      first_sn: first,
      last_sn: last,
      count: writer.heartbeat_message_counter,
    };

    let mut flags = BitFlags::<HEARTBEAT_Flags>::from_endianness(writer.endianness);

    if set_final_flag {
      flags.insert(HEARTBEAT_Flags::Final)
    }
    if set_liveliness_flag {
      flags.insert(HEARTBEAT_Flags::Liveliness)
    }

    let submessage = heartbeat.create_submessage(flags);
    match submessage {
      Some(sm) => self.submessages.push(sm),
      None => return self,
    }
    self
  }

  pub fn add_header_and_build(self, guid_prefix: GuidPrefix) -> Message {
    Message {
      header: Header {
        protocol_id: ProtocolId::default(),
        protocol_version: ProtocolVersion::THIS_IMPLEMENTATION,
        vendor_id: VendorId::THIS_IMPLEMENTATION,
        guid_prefix,
      },
      submessages: self.submessages,
    }
  }
}

#[cfg(test)]

mod tests {
  #![allow(non_snake_case)]
  use log::info;
  use speedy::Writable;

  use super::*;

  #[test]

  fn rtps_message_test_shapes_demo_message_deserialization() {
    // Data message should contain Shapetype values.
    // caprured with wireshark from shapes demo.
    // packet with INFO_DST, INFO_TS, DATA, HEARTBEAT
    let bits1 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x09, 0x01, 0x08, 0x00, 0x1a, 0x15, 0xf3, 0x5e, 0x00,
      0xcc, 0xfb, 0x13, 0x15, 0x05, 0x2c, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x07,
      0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x52, 0x45, 0x44, 0x00, 0x69, 0x00, 0x00, 0x00, 0x17, 0x00,
      0x00, 0x00, 0x1e, 0x00, 0x00, 0x00, 0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00,
      0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x5b, 0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00,
    ]);
    let rtps = Message::read_from_buffer(bits1.clone()).unwrap();
    info!("{:?}", rtps);

    let serialized = Bytes::from(
      rtps
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap(),
    );
    assert_eq!(bits1, serialized);
  }
  #[test]
  fn rtps_message_test_shapes_demo_DataP() {
    // / caprured with wireshark from shapes demo.
    // packet with DATA(p)
    let bits2 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x04, 0x01, 0x03, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 0x31,
      0xa2, 0x28, 0x20, 0x02, 0x08, 0x15, 0x05, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x01, 0x00, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00,
      0x03, 0x00, 0x00, 0x77, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x04, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x15, 0x00, 0x04, 0x00, 0x02, 0x04, 0x00, 0x00, 0x50, 0x00, 0x10,
      0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x00, 0x00,
      0x01, 0xc1, 0x16, 0x00, 0x04, 0x00, 0x01, 0x03, 0x00, 0x00, 0x44, 0x00, 0x04, 0x00, 0x3f,
      0x0c, 0x00, 0x00, 0x58, 0x00, 0x04, 0x00, 0x3f, 0x0c, 0x00, 0x00, 0x32, 0x00, 0x18, 0x00,
      0x01, 0x00, 0x00, 0x00, 0x9f, 0xa4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x50, 0x8e, 0xc9, 0x32, 0x00, 0x18, 0x00, 0x01, 0x00,
      0x00, 0x00, 0x9f, 0xa4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0xc0, 0xa8, 0x45, 0x14, 0x32, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00,
      0x9f, 0xa4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0xac, 0x11, 0x00, 0x01, 0x33, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xea, 0x1c,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xef,
      0xff, 0x00, 0x01, 0x31, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0x39, 0x30, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7f, 0x00, 0x00,
      0x01, 0x48, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0x39, 0x30, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7f, 0x00, 0x00, 0x01, 0x34,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0xb0, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00,
      0x02, 0x00, 0x08, 0x00, 0x2c, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
      0x00,
    ]);

    let rtps_data = Message::read_from_buffer(bits2.clone()).unwrap();

    let serialized_data = Bytes::from(
      rtps_data
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap(),
    );
    assert_eq!(bits2, serialized_data);
  }

  #[test]
  fn rtps_message_test_shapes_demo_info_TS_dataP() {
    // caprured with wireshark from shapes demo.
    // rtps packet with info TS and Data(p)
    let bits1 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x09, 0x01, 0x08, 0x00, 0x0e, 0x15, 0xf3, 0x5e, 0x00, 0x28,
      0x74, 0xd2, 0x15, 0x05, 0xa8, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x01, 0x00, 0xc7, 0x00,
      0x01, 0x00, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
      0x15, 0x00, 0x04, 0x00, 0x02, 0x03, 0x00, 0x00, 0x16, 0x00, 0x04, 0x00, 0x01, 0x0f, 0x00,
      0x00, 0x50, 0x00, 0x10, 0x00, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x01, 0xc1, 0x32, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xf4,
      0x1c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x0a, 0x50, 0x8e, 0x68, 0x31, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xf5, 0x1c, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x50,
      0x8e, 0x68, 0x02, 0x00, 0x08, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x58,
      0x00, 0x04, 0x00, 0x3f, 0x0c, 0x3f, 0x0c, 0x62, 0x00, 0x18, 0x00, 0x14, 0x00, 0x00, 0x00,
      0x66, 0x61, 0x73, 0x74, 0x72, 0x74, 0x70, 0x73, 0x50, 0x61, 0x72, 0x74, 0x69, 0x63, 0x69,
      0x70, 0x61, 0x6e, 0x74, 0x00, 0x01, 0x00, 0x00, 0x00,
    ]);

    let rtps = Message::read_from_buffer(bits1.clone()).unwrap();
    info!("{:?}", rtps);

    let serialized = Bytes::from(
      rtps
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap(),
    );
    assert_eq!(bits1, serialized);
  }

  #[test]
  fn rtps_message_test_shapes_demo_info_TS_AckNack() {
    // caprured with wireshark from shapes demo.
    // rtps packet with info TS three AckNacks
    let bits1 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x04, 0x01, 0x03, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d, 0x31,
      0xa2, 0x28, 0x20, 0x02, 0x08, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34,
      0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x03, 0x18, 0x00, 0x00, 0x00, 0x03, 0xc7, 0x00,
      0x00, 0x03, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x01, 0x00, 0x00, 0x00, 0x06, 0x03, 0x18, 0x00, 0x00, 0x00, 0x04, 0xc7, 0x00, 0x00, 0x04,
      0xc2, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x00, 0x06, 0x03, 0x18, 0x00, 0x00, 0x02, 0x00, 0xc7, 0x00, 0x02, 0x00, 0xc2, 0x00,
      0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    ]);

    let rtps = Message::read_from_buffer(bits1.clone()).unwrap();
    info!("{:?}", rtps);

    let serialized = Bytes::from(
      rtps
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap(),
    );
    assert_eq!(bits1, serialized);
  }

  #[test]
  fn rtps_message_info_ts_and_dataP() {
    // caprured with wireshark from shapes demo.
    // rtps packet with info TS and data(p)
    let bits1 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x09, 0x01, 0x08, 0x00, 0x0e, 0x15, 0xf3, 0x5e, 0x00, 0x28,
      0x74, 0xd2, 0x15, 0x05, 0xa8, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x01, 0x00, 0xc7, 0x00,
      0x01, 0x00, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
      0x15, 0x00, 0x04, 0x00, 0x02, 0x03, 0x00, 0x00, 0x16, 0x00, 0x04, 0x00, 0x01, 0x0f, 0x00,
      0x00, 0x50, 0x00, 0x10, 0x00, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x01, 0xc1, 0x32, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xf4,
      0x1c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x0a, 0x50, 0x8e, 0x68, 0x31, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xf5, 0x1c, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x50,
      0x8e, 0x68, 0x02, 0x00, 0x08, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x58,
      0x00, 0x04, 0x00, 0x3f, 0x0c, 0x3f, 0x0c, 0x62, 0x00, 0x18, 0x00, 0x14, 0x00, 0x00, 0x00,
      0x66, 0x61, 0x73, 0x74, 0x72, 0x74, 0x70, 0x73, 0x50, 0x61, 0x72, 0x74, 0x69, 0x63, 0x69,
      0x70, 0x61, 0x6e, 0x74, 0x00, 0x01, 0x00, 0x00, 0x00,
    ]);

    let rtps = Message::read_from_buffer(bits1.clone()).unwrap();
    info!("{:?}", rtps);

    let serialized = Bytes::from(
      rtps
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap(),
    );
    assert_eq!(bits1, serialized);
  }

  #[test]
  fn rtps_message_infoDST_infoTS_Data_w_heartbeat() {
    // caprured with wireshark from shapes demo.
    // rtps packet with InfoDST InfoTS Data(w) Heartbeat
    // This datamessage serialized payload maybe contains topic name (square) and
    // its type (shapetype) look https://www.omg.org/spec/DDSI-RTPS/2.3/PDF page 185
    let bits1 = Bytes::from_static(&[
      0x52, 0x54, 0x50, 0x53, 0x02, 0x03, 0x01, 0x0f, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x0c, 0x00, 0x01, 0x03, 0x00, 0x0c, 0x29, 0x2d,
      0x31, 0xa2, 0x28, 0x20, 0x02, 0x08, 0x09, 0x01, 0x08, 0x00, 0x12, 0x15, 0xf3, 0x5e, 0x00,
      0xc8, 0xa9, 0xfa, 0x15, 0x05, 0x0c, 0x01, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0xc7,
      0x00, 0x00, 0x03, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00,
      0x00, 0x2f, 0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00, 0xf5, 0x1c, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x50, 0x8e, 0x68, 0x50,
      0x00, 0x10, 0x00, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x01, 0xc1, 0x05, 0x00, 0x0c, 0x00, 0x07, 0x00, 0x00, 0x00, 0x53, 0x71, 0x75,
      0x61, 0x72, 0x65, 0x00, 0x00, 0x07, 0x00, 0x10, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x53, 0x68,
      0x61, 0x70, 0x65, 0x54, 0x79, 0x70, 0x65, 0x00, 0x00, 0x00, 0x70, 0x00, 0x10, 0x00, 0x01,
      0x0f, 0x99, 0x06, 0x78, 0x34, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02,
      0x5a, 0x00, 0x10, 0x00, 0x01, 0x0f, 0x99, 0x06, 0x78, 0x34, 0x00, 0x00, 0x01, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x01, 0x02, 0x60, 0x00, 0x04, 0x00, 0x5f, 0x01, 0x00, 0x00, 0x15, 0x00,
      0x04, 0x00, 0x02, 0x03, 0x00, 0x00, 0x16, 0x00, 0x04, 0x00, 0x01, 0x0f, 0x00, 0x00, 0x1d,
      0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x23, 0x00, 0x08, 0x00, 0xff, 0xff, 0xff, 0x7f,
      0xff, 0xff, 0xff, 0xff, 0x27, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x1b, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0x7f, 0xff, 0xff,
      0xff, 0xff, 0x1a, 0x00, 0x0c, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x9a,
      0x99, 0x99, 0x19, 0x2b, 0x00, 0x08, 0x00, 0xff, 0xff, 0xff, 0x7f, 0xff, 0xff, 0xff, 0xff,
      0x1f, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x25, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x07, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x03, 0xc7, 0x00, 0x00,
      0x03, 0xc2, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
      0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    ]);

    let rtps = Message::read_from_buffer(bits1.clone()).unwrap();
    info!("{:?}", rtps);

    let data_submessage = match &rtps.submessages[2] {
      SubMessage {
        header: _,
        body: SubmessageBody::Entity(EntitySubmessage::Data(d, _flags)),
      } => d,
      wtf => panic!("Unexpected message structure {:?}", wtf),
    };
    let serialized_payload = data_submessage
      .serialized_payload
      .as_ref()
      .unwrap()
      .value
      .clone();
    info!("{:x?}", serialized_payload);

    let serialized = Bytes::from(
      rtps
        .write_to_vec_with_ctx(Endianness::LittleEndian)
        .unwrap(),
    );
    assert_eq!(bits1, serialized);
  }

  // removed case test_RTPS_submessage_flags_helper , as it was cut-and-paste
  // from submessage_flag module - and obsoleted there.
}
