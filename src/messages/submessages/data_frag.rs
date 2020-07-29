use crate::messages::fragment_number::FragmentNumber;
use crate::messages::submessages::submessage_elements::parameter_list::ParameterList;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::structure::guid::EntityId;
use crate::structure::sequence_number::SequenceNumber;
use speedy::{Readable, Writable, Context, Writer, Reader};

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

impl DataFrag {
  pub fn new() -> DataFrag {
    DataFrag {
      reader_id: EntityId::default(),
      writer_id: EntityId::default(),
      writer_sn: SequenceNumber::default(),
      fragment_starting_num : FragmentNumber::default(),
      fragments_in_submessage : 0,
      data_size : 0,
      fragment_size : 0,
      inline_qos: None,
      serialized_payload: SerializedPayload::default(),
    }
  }
}

impl Default for DataFrag {
  fn default() -> Self {
    DataFrag::new()
  }
}

impl <'a, C: Context> Readable<'a, C> for DataFrag{
  fn read_from< R: Reader< 'a, C > >( reader: &mut R ) -> Result< Self, C::Error > {
    let mut dataFragMessage = DataFrag::default();
    //ExtraFlags This version of the protocol (2.3) should set all the bits in the extraFlags to zero
    //just ignore these
    reader.read_u16()?;
    //octets to InlineQos
    reader.read_u16()?;
    dataFragMessage.reader_id = reader.read_value()?;
    dataFragMessage.writer_id = reader.read_value()?;
    dataFragMessage.writer_sn = reader.read_value()?;
    dataFragMessage.fragment_starting_num = reader.read_value()?;
    dataFragMessage.fragments_in_submessage = reader.read_value()?;
    dataFragMessage.fragment_size = reader.read_value()?;
    dataFragMessage.data_size = reader.read_value()?;
    // TODO handle inlineQos ?????
    dataFragMessage.serialized_payload = reader.read_value()?;
    Ok(dataFragMessage)
  }
}


impl <C: Context> Writable<C> for DataFrag{
  fn write_to<'a, T: ?Sized + Writer< C>>(
    &'a self,
    writer: &mut T
) -> Result<(), C::Error>{
    writer.write_u16(0)?;
    if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() > 0{
      println!("self.inline_qos {:?}", self.inline_qos);
      todo!()
    }else if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() ==  0{
      writer.write_u16(24)?;
    }
    else if self.inline_qos.is_none(){
      writer.write_u16(24)?;
    }
    writer.write_value(&self.reader_id)?;
    writer.write_value(&self.writer_id)?;
    writer.write_value(&self.writer_sn)?;
    writer.write_value(&self.fragment_starting_num)?;
    writer.write_value(&self.fragments_in_submessage)?;
    writer.write_value(&self.fragment_size)?;
    writer.write_value(&self.data_size)?;
    if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() > 0{
      writer.write_value(&self.inline_qos)?;
    }
    writer.write_value(&self.serialized_payload)?;
    Ok(())
  }
}

