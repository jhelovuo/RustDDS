use crate::messages::submessages::submessage_elements::parameter_list::ParameterList;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::structure::guid::EntityId;
use crate::structure::sequence_number::SequenceNumber;
use speedy::{Readable, Writable, Context, Writer, Reader};

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
}

impl Default for Data {
  fn default() -> Self {
    Data::new()
  }
}

impl <'a, C: Context> Readable<'a, C> for Data{
  fn read_from< R: Reader< 'a, C > >( reader: &mut R ) -> Result< Self, C::Error > {
    let mut dataMessage = Data::default();
    //ExtraFlags This version of the protocol (2.3) should set all the bits in the extraFlags to zero
    //just ignore these
    reader.read_u16()?;
    //octets to InlineQos
    reader.read_u16()?;
    dataMessage.reader_id = reader.read_value()?;
    dataMessage.writer_id = reader.read_value()?;
    dataMessage.writer_sn = reader.read_value()?;

    // TODO INLINE QOS ??

    dataMessage.serialized_payload = reader.read_value()?;
    Ok(dataMessage)


  }
}

impl <C: Context> Writable<C> for Data{
  fn write_to<'a, T: ?Sized + Writer< C>>(
    &'a self,
    writer: &mut T
) -> Result<(), C::Error>{
    //This version of the protocol (2.3) should set all the bits in the extraFlags to zero
    writer.write_u16(0)?;
    //The octetsToInlineQos field contains the number of octets starting from the first octet immediately following
    //this field until the first octet of the inlineQos SubmessageElement. If the inlineQos SubmessageElement is not
    //present (i.e., the InlineQosFlag is not set), then octetsToInlineQos contains the offset to the next field after
    //the inlineQos.

    // TODO INLINE QOS ??
    
    if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() > 0{
      println!("self.inline_qos {:?}", self.inline_qos);
      todo!()
    }else if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() ==  0{
      writer.write_u16(16)?;
    }
    else if self.inline_qos.is_none(){
      writer.write_u16(16)?;
    }
    writer.write_value(&self.reader_id)?;
    writer.write_value(&self.writer_id)?;
    writer.write_value(&self.writer_sn)?;
    if self.inline_qos.is_some() && self.inline_qos.as_ref().unwrap().parameters.len() > 0{
      writer.write_value(&self.inline_qos)?;
    }
    writer.write_value(&self.serialized_payload)?;
    Ok(())
}
 
}
