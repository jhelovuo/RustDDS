use std::{cmp::min, io};

use bytes::{Bytes, BytesMut};
use speedy::{Context, Readable, Writable, Writer};
use serde::{Deserialize, Serialize};
use byteorder::ReadBytesExt;
use log::warn;

/// Used to identify serialization format of payload data over RTPS.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Readable, Writable, Serialize, Deserialize)]
pub struct RepresentationIdentifier {
  bytes: [u8; 2],
}

impl RepresentationIdentifier {
  // Numeric values are from RTPS spec v2.3 Section 10.5 , Table 10.3
  pub const CDR_BE: Self = Self {
    bytes: [0x00, 0x00],
  };
  pub const CDR_LE: Self = Self {
    bytes: [0x00, 0x01],
  };

  pub const PL_CDR_BE: Self = Self {
    bytes: [0x00, 0x02],
  };
  pub const PL_CDR_LE: Self = Self {
    bytes: [0x00, 0x03],
  };

  pub const CDR2_BE: Self = Self {
    bytes: [0x00, 0x10],
  };
  pub const CDR2_LE: Self = Self {
    bytes: [0x00, 0x11],
  };

  pub const PL_CDR2_BE: Self = Self {
    bytes: [0x00, 0x12],
  };
  pub const PL_CDR2_LE: Self = Self {
    bytes: [0x00, 0x13],
  };

  pub const D_CDR_BE: Self = Self {
    bytes: [0x00, 0x14],
  };
  pub const D_CDR_LE: Self = Self {
    bytes: [0x00, 0x15],
  };

  pub const XML: Self = Self {
    bytes: [0x00, 0x04],
  };

  // Reads two bytes to form a `RepresentationIdentifier`
  pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
    let mut reader = io::Cursor::new(bytes);
    Ok(Self {
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
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SerializedPayload {
  pub representation_identifier: RepresentationIdentifier,
  pub representation_options: [u8; 2], // Not used. Send as zero, ignore on receive.
  pub value: Bytes,
}

// header length
// 2 bytes for representation identifier
// + 2 bytes for representation options
const H_LEN: usize = 2 + 2;

impl SerializedPayload {
  #[cfg(test)]
  pub fn new(rep_id: RepresentationIdentifier, payload: Vec<u8>) -> Self {
    Self {
      representation_identifier: rep_id,
      representation_options: [0, 0],
      value: Bytes::from(payload),
    }
  }

  pub fn new_from_bytes(rep_id: RepresentationIdentifier, payload: Bytes) -> Self {
    Self {
      representation_identifier: rep_id,
      representation_options: [0, 0],
      value: payload,
    }
  }

  /// serialized size in bytes
  pub fn len_serialized(&self) -> usize {
    H_LEN + self.value.len()
  }

  // a slice of serialized data
  // This has a lot of H_LEN offsets, because the data to be sliced
  // is header + value
  pub fn bytes_slice(&self, from: usize, to_before: usize) -> Bytes {
    // sanitize inputs. These are unsigned values, so always at least zero.
    let to_before = min(to_before, self.value.len() + H_LEN);
    let from = min(from, to_before);

    if from >= H_LEN {
      // no need to copy, can return a slice
      self.value.slice(from - H_LEN..to_before - H_LEN)
    } else {
      // We need to copy the payload on order to prefix with header
      let mut b = BytesMut::with_capacity(to_before);
      b.extend_from_slice(&self.representation_identifier.bytes);
      b.extend_from_slice(&self.representation_options);
      assert_eq!(b.len(), H_LEN);
      if to_before > H_LEN {
        b.extend_from_slice(&self.value.slice(..to_before - H_LEN));
      }
      b.freeze().slice(from..to_before)
    }
  }

  // Implement deserialization here, because Speedy just makes it difficult.
  pub fn from_bytes(bytes: &Bytes) -> io::Result<Self> {
    let mut reader = io::Cursor::new(&bytes);
    let representation_identifier = RepresentationIdentifier {
      bytes: [reader.read_u8()?, reader.read_u8()?],
    };
    let representation_options = [reader.read_u8()?, reader.read_u8()?];
    let value = if bytes.len() >= H_LEN {
      bytes.slice(H_LEN..)
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

    Ok(Self {
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
