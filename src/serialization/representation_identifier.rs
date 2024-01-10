use std::io;

use speedy::{Readable, Writable};
use byteorder::ReadBytesExt;

/// Used to identify serialization format of payload data over RTPS.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Readable, Writable)]
pub struct RepresentationIdentifier {
  pub(crate) bytes: [u8; 2], // semi-public for serialization elsewhere
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

  // [0x00,0x04] defined below
  // [0x00,0x05] is not defined, as far as we know

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

  // XML
  pub const XML: Self = Self {
    bytes: [0x00, 0x04],
  };


  // The following are from 
  // "Extensible and Dynamic Topic Types for DDS" (DDS X-Types v 1.2) spec, Table 60,
  // Section 7.6.2.1.2 Use of the RTPS Encapsulation Identifier

  // Table says "CDR2_BE", but that name is already taken.
  pub const XCDR2_BE: Self = Self {
    bytes: [0x00, 0x06],
  };

  pub const XCDR2_LE: Self = Self {
    bytes: [0x00, 0x07],
  };

  pub const D_CDR2_BE: Self = Self {
    bytes: [0x00, 0x08],
  };

  pub const D_CDR2_LE: Self = Self {
    bytes: [0x00, 0x09],
  };

  // Table says "PL_CDR2_BE", but name is already taken
  pub const PL_XCDR2_BE: Self = Self {
    bytes: [0x00, 0x0a],
  };
  // Table says "PL_CDR_LE" (no "2"), but this is likely a typo in the spec.
  pub const PL_XCDR2_LE: Self = Self {
    bytes: [0x00, 0x0b],
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
