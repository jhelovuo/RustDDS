use speedy::{Readable, Writable, Writer, Reader, Context};
//use byteorder::{ByteOrder, BigEndian };

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
}

impl RepresentationIdentifier {
  pub fn try_from_u16(ri_raw : u16) -> Result<RepresentationIdentifier,u16> {
    RepresentationIdentifier::try_from_primitive(ri_raw).map_err( |e| e.number )
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
  pub representation_options: u16, // Not used. Send as zero, ignore on receive.
  pub value: Vec<u8>,
}

impl SerializedPayload {
  
  pub fn new(rep_id:RepresentationIdentifier, payload:Vec<u8>) -> SerializedPayload {
    SerializedPayload {
      representation_identifier: rep_id as u16,
      representation_options: 0,
      value: payload,
    }
  }
}

// TODO: Remove this. It does not make sense to have a default value for this.
impl Default for SerializedPayload {
  fn default() -> SerializedPayload {
    SerializedPayload::new(RepresentationIdentifier::CDR_LE, b"fake data".to_vec() )
  }
}

impl<'a, C: Context> Readable<'a, C> for SerializedPayload {
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    //let mut payload = SerializedPayload::default();
    //let mut representationIentifierBytes: Vec<u8> = vec![];

    // Speedy is now stupid here. We get a speedy reader, but it has an unknown endianness.
    // 
    let rep_id_high = reader.read_u8()?;
    let rep_id_low = reader.read_u8()?;
    let rep_id = ((rep_id_high as u16) << 8) | (rep_id_low as u16); // construct as big-endian

    /*representationIentifierBytes.push();
    representationIentifierBytes.push(reader.read_u8()?);
    let u_16 = BigEndian::read_u16(&representationIentifierBytes);
    payload.representation_identifier = SerializedPayload::representation_identifier_from(u_16); */
    let ro : u16 = reader.read_value()?;

    let mut value = Vec::with_capacity(128); // just a guess to avoid too much reallocation 
    loop {
      let res = reader.read_u8();
      match res {
        Ok(by) => value.push(by),
        Err(_error) => break,
      };
    }
    Ok(SerializedPayload {
      representation_identifier: rep_id,
      representation_options: ro,
      value, 
    })
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
