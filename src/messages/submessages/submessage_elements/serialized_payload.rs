use speedy::{Writable, Writer, Context};
use std::io;
use std::io::Read;
use byteorder::{ReadBytesExt, BigEndian};

use num_enum::{TryFromPrimitive, IntoPrimitive};
//use std::convert::TryFrom;

#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive, IntoPrimitive)]
#[repr(u16)]
pub enum RepresentationIdentifier {
  // Numeric values are from RTPS spec v2.3 Section 10.5 , Table 10.3
  CDR_BE = 0,
  CDR_LE = 1,
  PL_CDR_BE = 2,
  PL_CDR_LE = 3,
  CDR2_BE = 0x0010,
  CDR2_LE = 0x0011,
  PL_CDR2_BE = 0x0012,
  PL_CDR2_LE = 0x0013,
  D_CDR_BE = 0x0014,
  D_CDR_LE = 0x0015,
  XML = 0x0004,

  INVALID = 0xffff,
}

impl RepresentationIdentifier {
  pub fn try_from_u16(ri_raw: u16) -> Result<RepresentationIdentifier, u16> {
    RepresentationIdentifier::try_from_primitive(ri_raw).map_err(|e| e.number)
  }
}

/// A SerializedPayload submessage element contains the serialized representation of
/// either value of an application-defined data-object or
/// the value of the key that uniquely identifies the data-object
/// See RTPS spec v2.3 section 10.
/// Standard representation identifer values are defined in sections 10.2 - 10.5
/// representation_options "shall be interpreted in the context of the
/// RepresentationIdentifier, such that each RepresentationIdentifier may define the
/// representation_options that it requires." and "The [2.3] version of the protocol
/// does not use the representation_options: The sender shall set the representation_options
/// to zero. The receiver shall ignore the value of the representation_options."
#[derive(Debug, PartialEq, Clone)]
pub struct SerializedPayload {
  pub representation_identifier: u16, // This is u16, not RepresentationIdentifier, because we need to be able to deserialize whatever is on the wire
  pub representation_options: [u8; 2], // Not used. Send as zero, ignore on receive.
  pub value: Vec<u8>,
}

impl SerializedPayload {
  pub fn new(rep_id: RepresentationIdentifier, payload: Vec<u8>) -> SerializedPayload {
    SerializedPayload {
      representation_identifier: rep_id as u16,
      representation_options: [0, 0],
      value: payload,
    }
  }

  // Implement deserialization here, because Speedy just makes it difficult.
  pub fn from_bytes(bytes: &[u8]) -> io::Result<SerializedPayload> {
    let mut reader = io::Cursor::new(bytes);
    let representation_identifier = reader.read_u16::<BigEndian>()?;
    let representation_options = [reader.read_u8()?, reader.read_u8()?];
    let mut value = Vec::with_capacity(bytes.len() - 4); // still length == 0
    reader.read_to_end(&mut value)?;

    Ok(SerializedPayload {
      representation_identifier,
      representation_options,
      value,
    })
  }
}

impl<C: Context> Writable<C> for SerializedPayload {
  fn write_to<'a, T: ?Sized + Writer<C>>(&'a self, writer: &mut T) -> Result<(), C::Error> {
    let primitive: u16 = self.representation_identifier.into();
    let bigEndianRepresentation = primitive.to_be_bytes();
    writer.write_u8(bigEndianRepresentation[0])?;
    writer.write_u8(bigEndianRepresentation[1])?;
    writer.write_u8(self.representation_options[0])?;
    writer.write_u8(self.representation_options[1])?;
    for element in &*self.value {
      writer.write_u8(*element)?;
    }
    Ok(())
  }
}
