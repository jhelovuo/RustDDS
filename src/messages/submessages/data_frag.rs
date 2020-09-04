use crate::messages::fragment_number::FragmentNumber;
use crate::messages::submessages::submessage_elements::parameter_list::ParameterList;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::structure::guid::EntityId;
use crate::structure::sequence_number::SequenceNumber;

use crate::messages::submessages::submessages::*;


use speedy::{Context, Writer, Readable, Writable, Error};
use enumflags2::BitFlags;

use std::io;


/// The DataFrag Submessage extends the Data Submessage by enabling the
/// serializedData to be fragmented and sent as multiple DataFrag Submessages.
/// The fragments contained in the DataFrag Submessages are then re-assembled by
/// the RTPS Reader.
#[derive(Debug, PartialEq)]
pub struct DataFrag {
  /// Identifies the RTPS Reader entity that is being informed of the change
  /// to the data-object.
  pub reader_id: EntityId,

  /// Identifies the RTPS Writer entity that made the change to the
  /// data-object.
  pub writer_id: EntityId,

  /// Uniquely identifies the change and the relative order for all changes
  /// made by the RTPS Writer identified by the writerGuid.
  /// Each change gets a consecutive sequence number.
  /// Each RTPS Writer maintains is own sequence number.
  pub writer_sn: SequenceNumber,

  /// Indicates the starting fragment for the series of fragments in
  /// serialized_data. Fragment numbering starts with number 1.
  pub fragment_starting_num: FragmentNumber,

  /// The number of consecutive fragments contained in this Submessage,
  /// starting at fragment_starting_num.
  pub fragments_in_submessage: u16,

  /// The total size in bytes of the original data before fragmentation.
  pub data_size: u32,

  /// The size of an individual fragment in bytes. The maximum fragment size
  /// equals 64K.
  pub fragment_size: u16,

  /// Contains QoS that may affect the interpretation of the message.
  /// Present only if the InlineQosFlag is set in the header.
  pub inline_qos: Option<ParameterList>,

  /// Encapsulation of a consecutive series of fragments, starting at
  /// fragment_starting_num for a total of fragments_in_submessage.
  /// Represents part of the new value of the data-object
  /// after the change. Present only if either the DataFlag or the KeyFlag are
  /// set in the header. Present only if DataFlag is set in the header.
  pub serialized_payload: SerializedPayload,
}

impl<'a> DataFrag {

  pub fn deserialize(buffer: &'a [u8], flags: BitFlags<DATAFRAG_Flags>) -> io::Result<DataFrag> {
    let mut cursor = io::Cursor::new(buffer);
    let endianness = endianness_flag(flags.bits());
    let map_speedy_err = |p: Error| io::Error::new(io::ErrorKind::Other, p);

    let _extra_flags =
      u16::read_from_stream_with_ctx(endianness, &mut cursor).map_err(map_speedy_err)?;
    let octets_to_inline_qos =
      u16::read_from_stream_with_ctx(endianness, &mut cursor).map_err(map_speedy_err)?;
    let reader_id =
      EntityId::read_from_stream_with_ctx(endianness, &mut cursor).map_err(map_speedy_err)?;
    let writer_id =
      EntityId::read_from_stream_with_ctx(endianness, &mut cursor).map_err(map_speedy_err)?;
    let writer_sn =
      SequenceNumber::read_from_stream_with_ctx(endianness, &mut cursor).map_err(map_speedy_err)?;
    let fragment_starting_num =
      FragmentNumber::read_from_stream_with_ctx(endianness, &mut cursor).map_err(map_speedy_err)?;
    let fragments_in_submessage = 
      u16::read_from_stream_with_ctx(endianness, &mut cursor).map_err(map_speedy_err)?;
    let fragment_size =
      u16::read_from_stream_with_ctx(endianness, &mut cursor).map_err(map_speedy_err)?;
    let data_size =
      u32::read_from_stream_with_ctx(endianness, &mut cursor).map_err(map_speedy_err)?;

    let expect_qos = flags.contains(DATAFRAG_Flags::InlineQos);
    //let expect_key = flags.contains(DATAFRAG_Flags::Key);

    // Skip any possible fields we do not know about.
    let rtps_v23_header_size: u16 = 7*4; 
    let extra_octets = octets_to_inline_qos - rtps_v23_header_size;
    if octets_to_inline_qos < rtps_v23_header_size { 
      return Err(io::Error::new(io::ErrorKind::Other, "DataFrag has too low octetsToInlineQos")) 
    }
    cursor.set_position(cursor.position() + extra_octets as u64);

    let inline_qos = if expect_qos {  
        Some(ParameterList::read_from_stream_with_ctx(endianness, &mut cursor)
              .map_err(map_speedy_err)? )
      } 
      else { None };

    // Payload should be always present, be it data or key fragments.
    let serialized_payload = SerializedPayload::from_bytes(&buffer[cursor.position() as usize..])?;

    Ok( DataFrag { reader_id, writer_id, writer_sn, fragment_starting_num, fragments_in_submessage,
                  data_size, fragment_size, inline_qos, serialized_payload,
        })
  }

}

impl Default for DataFrag {
  fn default() -> Self {
    DataFrag {
      reader_id: EntityId::default(),
      writer_id: EntityId::default(),
      writer_sn: SequenceNumber::default(),
      fragment_starting_num: FragmentNumber::default(),
      fragments_in_submessage: 0,
      data_size: 0,
      fragment_size: 0,
      inline_qos: None,
      serialized_payload: SerializedPayload::default(),
    }
  }
}

impl<C: Context> Writable<C> for DataFrag {
  fn write_to<'a, T: ?Sized + Writer<C>>(&'a self, writer: &mut T) -> Result<(), C::Error> {
    writer.write_u16(0)?;
    if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() > 0 {
      println!("self.inline_qos {:?}", self.inline_qos);
      todo!()
    } else if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() == 0 {
      writer.write_u16(24)?;
    } else if self.inline_qos.is_none() {
      writer.write_u16(24)?;
    }
    writer.write_value(&self.reader_id)?;
    writer.write_value(&self.writer_id)?;
    writer.write_value(&self.writer_sn)?;
    writer.write_value(&self.fragment_starting_num)?;
    writer.write_value(&self.fragments_in_submessage)?;
    writer.write_value(&self.fragment_size)?;
    writer.write_value(&self.data_size)?;
    if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() > 0 {
      writer.write_value(&self.inline_qos)?;
    }
    writer.write_value(&self.serialized_payload)?;
    Ok(())
  }
}
