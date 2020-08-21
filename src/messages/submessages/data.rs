use crate::messages::submessages::submessage_elements::parameter_list::ParameterList;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::structure::guid::EntityId;
use crate::structure::sequence_number::SequenceNumber;
use speedy::{Readable, Writable, Context, Writer, Endianness};

/// This Submessage is sent from an RTPS Writer (NO_KEY or WITH_KEY)
/// to an RTPS Reader (NO_KEY or WITH_KEY)
///
/// The Submessage notifies the RTPS Reader of a change to
/// a data-object belonging to the RTPS Writer. The possible changes
/// include both changes in value as well as changes to the lifecycle
/// of the data-object.
#[derive(Debug, PartialEq, Clone)]
pub struct Data {
  /// Identifies the RTPS Reader entity that is being informed of the change
  /// to the data-object.
  pub reader_id: EntityId,

  /// Identifies the RTPS Writer entity that made the change to the
  /// data-object.
  pub writer_id: EntityId,

  /// Uniquely identifies the change and the relative order for all changes
  /// made by the RTPS Writer identified by the writerGuid. Each change
  /// gets a consecutive sequence number. Each RTPS Writer maintains is
  /// own sequence number.
  pub writer_sn: SequenceNumber,

  /// Contains QoS that may affect the interpretation of the message.
  /// Present only if the InlineQosFlag is set in the header.
  pub inline_qos: Option<ParameterList>,

  /// If the DataFlag is set, then it contains the encapsulation of
  /// the new value of the data-object after the change.
  /// If the KeyFlag is set, then it contains the encapsulation of
  /// the key of the data-object the message refers to.
  pub serialized_payload: SerializedPayload,
}

impl Data {
  pub fn new() -> Data {
    Data {
      reader_id: EntityId::default(),
      writer_id: EntityId::default(),
      writer_sn: SequenceNumber::default(),
      inline_qos: None,
      serialized_payload: SerializedPayload::default(),
    }
  }

  /// DATA submessage cannot be speedy Readable because deserializing this requires info from submessage header.
  /// Required iformation is  expect_qos and expect_payload whish are told on submessage headerflags.
  pub fn deserialize_data(
    buffer: &Vec<u8>,
    _context: Endianness,
    expect_qos: bool,
    expect_payload: bool,
  ) -> Data {
    let mut d = Data::new();
    let _extra_flags = &buffer[0..2];
    let octets_to_inline_qos = u16::read_from_buffer(&buffer[2..4]).unwrap();
    let octets_to_inline_qos_usize = octets_to_inline_qos as usize;
    let reader_id = &buffer[4..8];
    let writer_id = &buffer[8..12];
    let sequence_number = &buffer[12..20];

    if expect_qos {
      let QoS_list_length = u32::read_from_buffer(
        &buffer[octets_to_inline_qos_usize..(octets_to_inline_qos_usize + 4)],
      )
      .unwrap() as usize;
      d.inline_qos = Some(
        ParameterList::read_from_buffer(
          &buffer[octets_to_inline_qos_usize..octets_to_inline_qos_usize + QoS_list_length],
        )
        .unwrap(),
      );
    }
    if expect_payload && !expect_qos {
      d.serialized_payload =
        SerializedPayload::read_from_buffer(&buffer[octets_to_inline_qos_usize + 4..buffer.len()])
          .unwrap();
    }
    if expect_payload && expect_qos {
      let QoS_list_length = u32::read_from_buffer(
        &buffer[octets_to_inline_qos_usize..(octets_to_inline_qos_usize + 4)],
      )
      .unwrap() as usize;
      d.serialized_payload = SerializedPayload::read_from_buffer(
        &buffer[octets_to_inline_qos_usize + 4 + QoS_list_length..buffer.len()],
      )
      .unwrap();
    }

    d.reader_id = EntityId::read_from_buffer(reader_id).unwrap();
    d.writer_id = EntityId::read_from_buffer(writer_id).unwrap();
    d.writer_sn = SequenceNumber::read_from_buffer(sequence_number).unwrap();
    return d;
  }
}

impl Default for Data {
  fn default() -> Self {
    Data::new()
  }
}

impl<C: Context> Writable<C> for Data {
  fn write_to<'a, T: ?Sized + Writer<C>>(&'a self, writer: &mut T) -> Result<(), C::Error> {
    //This version of the protocol (2.3) should set all the bits in the extraFlags to zero
    writer.write_u16(0)?;
    //The octetsToInlineQos field contains the number of octets starting from the first octet immediately following
    //this field until the first octet of the inlineQos SubmessageElement. If the inlineQos SubmessageElement is not
    //present (i.e., the InlineQosFlag is not set), then octetsToInlineQos contains the offset to the next field after
    //the inlineQos.

    if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() > 0 {
      writer.write_value(&self.inline_qos)?;
    } else if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() == 0 {
      writer.write_u16(16)?;
    } else if self.inline_qos.is_none() {
      writer.write_u16(16)?;
    }
    writer.write_value(&self.reader_id)?;
    writer.write_value(&self.writer_id)?;
    writer.write_value(&self.writer_sn)?;
    if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() > 0 {
      writer.write_value(&self.inline_qos)?;
    }
    writer.write_value(&self.serialized_payload)?;
    Ok(())
  }
}
