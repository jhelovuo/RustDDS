use speedy::{Readable, Writable, Writer, Reader, Context};
use byteorder::{ByteOrder, BigEndian};

use num_enum::{TryFromPrimitive, IntoPrimitive};
use std::convert::TryFrom;

#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive, IntoPrimitive)]
#[repr(u16)]
pub enum RepresentationIdentifier {
  CDR_BE = 0,
  CDR_LE = 1,
  PL_CDR_BE = 2,
  PL_CDR_LE = 3,
  CDR2_BE = 16,
  CDR2_LE = 17,
  PL_CDR2_BE = 18,
  PL_CDR2_LE = 19,
  D_CDR_BE = 20,
  D_CDR_LE = 21,
  XML = 4,
}

/// A SerializedPayload contains the serialized representation of
/// either value of an application-defined data-object or
/// the value of the key that uniquely identifies the data-object
#[derive(Debug, PartialEq, Clone)]
pub struct SerializedPayload {
  pub representation_identifier: RepresentationIdentifier,
  pub representation_options: u16,
  pub value: Vec<u8>,
}

impl SerializedPayload {
  pub fn new() -> SerializedPayload {
    SerializedPayload {
      representation_identifier: RepresentationIdentifier::CDR_LE,
      representation_options: 0,
      value: vec![],
    }
  }
  pub fn representation_identifier_from(u_16: u16) -> RepresentationIdentifier {
    // TODO: handle unwrap
    RepresentationIdentifier::try_from(u_16).unwrap()
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
    let u_16 = BigEndian::read_u16(&representationIentifierBytes);
    payload.representation_identifier = SerializedPayload::representation_identifier_from(u_16);
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
    let primitive: u16 = self.representation_identifier.into();
    let bigEndianRepresentation = primitive.clone().to_be_bytes();
    writer.write_u8(bigEndianRepresentation[0])?;
    writer.write_u8(bigEndianRepresentation[1])?;
    writer.write_u16(self.representation_options)?;
    for element in &*self.value {
      writer.write_u8(*element)?;
    }
    Ok(())
  }
}
