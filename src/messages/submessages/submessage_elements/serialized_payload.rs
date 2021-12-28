use std::io;

use bytes::Bytes;
use speedy::{Context, Readable, Writable, Writer};
use byteorder::ReadBytesExt;
use log::warn;

/// Used to identify serialization format of payload data over RTPS.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Readable, Writable)]
pub struct RepresentationIdentifier {
  bytes: [u8; 2],
}

impl RepresentationIdentifier {
  // Numeric values are from RTPS spec v2.3 Section 10.5 , Table 10.3
  pub const CDR_BE: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x00],
  };
  pub const CDR_LE: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x01],
  };

  pub const PL_CDR_BE: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x02],
  };
  pub const PL_CDR_LE: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x03],
  };

  pub const CDR2_BE: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x10],
  };
  pub const CDR2_LE: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x11],
  };

  pub const PL_CDR2_BE: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x12],
  };
  pub const PL_CDR2_LE: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x13],
  };

  pub const D_CDR_BE: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x14],
  };
  pub const D_CDR_LE: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x15],
  };

  pub const XML: RepresentationIdentifier = RepresentationIdentifier {
    bytes: [0x00, 0x04],
  };

  // Reads two bytes to form a `RepresentationIdentifier`
  pub fn from_bytes(bytes: &[u8]) -> io::Result<RepresentationIdentifier> {
    let mut reader = io::Cursor::new(bytes);
    Ok(RepresentationIdentifier {
      bytes: [reader.read_u8()?, reader.read_u8()?],
    })
  }

  pub fn to_bytes(self) -> [u8; 2] {
    self.bytes
  }
}
/*
impl<'a, C: Context> Readable<'a, C> for RepresentationIdentifier {
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
    let mut r = RepresentationIdentifier { bytes: [0,0]};
    reader.read_bytes(r.bytes)?;
    r
  }
}
*/

/// A SerializedPayload submessage element contains the serialized
/// representation of either value of an application-defined data-object or
/// the value of the key that uniquely identifies the data-object
/// See RTPS spec v2.3 section 10.
/// Standard representation identifer values are defined in sections 10.2 - 10.5
/// representation_options "shall be interpreted in the context of the
/// RepresentationIdentifier, such that each RepresentationIdentifier may define
/// the representation_options that it requires." and "The [2.3] version of the
/// protocol does not use the representation_options: The sender shall set the
/// representation_options to zero. The receiver shall ignore the value of the
/// representation_options."
#[derive(Debug, PartialEq, Clone)]
pub struct SerializedPayload {
  pub representation_identifier: RepresentationIdentifier,
  pub representation_options: [u8; 2], // Not used. Send as zero, ignore on receive.
  pub value: Bytes,
}

impl SerializedPayload {
  pub fn new(rep_id: RepresentationIdentifier, payload: Vec<u8>) -> SerializedPayload {
    SerializedPayload {
      representation_identifier: rep_id,
      representation_options: [0, 0],
      value: Bytes::from(payload),
    }
  }

  pub fn new_from_bytes(rep_id: RepresentationIdentifier, payload: Bytes) -> SerializedPayload {
    SerializedPayload {
      representation_identifier: rep_id,
      representation_options: [0, 0],
      value: payload,
    }
  }

  // Implement deserialization here, because Speedy just makes it difficult.
  pub fn from_bytes(bytes: &Bytes) -> io::Result<SerializedPayload> {
    let mut reader = io::Cursor::new(&bytes);
    let representation_identifier = RepresentationIdentifier {
      bytes: [reader.read_u8()?, reader.read_u8()?],
    };
    let representation_options = [reader.read_u8()?, reader.read_u8()?];
    let value = if bytes.len() >= 4 {
      bytes.slice(4..) // split_off 4 bytes at beginning: rep_id &
                       // rep_optins
    } else {
      warn!(
        "DATA submessage was smaller than submessage header: {:?}",
        bytes
      );
      return Err(io::Error::new(
        io::ErrorKind::Other,
        "Too short DATA submessage.",
      ));
    };

    Ok(SerializedPayload {
      representation_identifier,
      representation_options,
      value,
    })
  }
}

impl<C: Context> Writable<C> for SerializedPayload {
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    writer.write_u8(self.representation_identifier.bytes[0])?;
    writer.write_u8(self.representation_identifier.bytes[1])?;
    writer.write_u8(self.representation_options[0])?;
    writer.write_u8(self.representation_options[1])?;
    writer.write_bytes(&self.value)?;
    Ok(())
  }
}
