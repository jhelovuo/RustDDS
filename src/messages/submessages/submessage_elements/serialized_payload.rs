use speedy::{Readable, Writable, Writer, Reader, Context};
use byteorder::{ByteOrder, BigEndian};

/// A SerializedPayload contains the serialized representation of
/// either value of an application-defined data-object or
/// the value of the key that uniquely identifies the data-object
#[derive(Debug, PartialEq, Clone)]
pub struct SerializedPayload {
  pub representation_identifier: u16,
  pub representation_options: u16,
  pub value: Vec<u8>,
}

impl SerializedPayload {
  pub fn new() -> SerializedPayload {
    SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: vec![],
    }
  }
}
impl Default for SerializedPayload {
  fn default() -> SerializedPayload {
    SerializedPayload::new()
  }
}

impl<'a, C: Context> Readable<'a, C> for SerializedPayload {
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let mut payload = SerializedPayload::default();
    let mut representationIentifierBytes: Vec<u8> = vec![];
    representationIentifierBytes.push(reader.read_u8()?);
    representationIentifierBytes.push(reader.read_u8()?);
    payload.representation_identifier = BigEndian::read_u16(&representationIentifierBytes);
    payload.representation_options = reader.read_value()?;

    loop {
      let res = reader.read_u8();
      let by = match res {
        Ok(by) => by,
        Err(_error) => {
          break;
        }
      };
      payload.value.push(by);
    }
    Ok(payload)
  }
}

impl<C: Context> Writable<C> for SerializedPayload {
  fn write_to<'a, T: ?Sized + Writer<C>>(&'a self, writer: &mut T) -> Result<(), C::Error> {
    let bigEndianRepresentation = self.representation_identifier.to_be_bytes();
    writer.write_u8(bigEndianRepresentation[0])?;
    writer.write_u8(bigEndianRepresentation[1])?;
    writer.write_u16(self.representation_options)?;
    for element in &self.value {
      writer.write_u8(*element)?;
    }
    Ok(())
  }
}
