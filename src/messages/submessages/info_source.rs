use speedy::{Readable, Writable};
#[cfg(feature = "security")]
use enumflags2::BitFlags;

use crate::{
  messages::{
    header::Header, protocol_id::ProtocolId, protocol_version::ProtocolVersion, vendor_id::VendorId,
  },
  structure::guid::GuidPrefix,
};
#[cfg(feature = "security")]
use super::{
  submessage::InterpreterSubmessage, submessage_flag::INFOSOURCE_Flags,
  submessage_kind::SubmessageKind, submessages::SubmessageHeader,
};
#[cfg(feature = "security")]
use crate::rtps::{Submessage, SubmessageBody};

/// This message modifies the logical source of the Submessages
/// that follow.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Readable, Writable)]
pub struct InfoSource {
  /// DDSI-RTPS 2.5 specifies an unused long here in 9.4.5.11
  pub unused: u32,

  /// Indicates the protocol used to encapsulate subsequent Submessages
  pub protocol_version: ProtocolVersion,

  /// Indicates the VendorId of the vendor that
  /// encapsulated subsequent Submessage
  pub vendor_id: VendorId,

  /// Identifies the Participant that is the container of the RTPS Writer
  /// entities that are the source of the Submessages that follow
  pub guid_prefix: GuidPrefix,
}

impl InfoSource {
  #[cfg(feature = "security")] // currently otherwise unused, clippy warns about this
  pub fn len_serialized(&self) -> usize {
    std::mem::size_of::<u32>()
      + std::mem::size_of::<ProtocolVersion>()
      + std::mem::size_of::<VendorId>()
      + std::mem::size_of::<GuidPrefix>()
  }

  #[cfg(feature = "security")] // currently otherwise unused
  pub fn create_submessage(self, flags: BitFlags<INFOSOURCE_Flags>) -> Submessage {
    Submessage {
      header: SubmessageHeader {
        kind: SubmessageKind::INFO_SRC,
        flags: flags.bits(),
        content_length: self.len_serialized() as u16,
      },
      body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoSource(self, flags)),
      original_bytes: None,
    }
  }
}

impl From<Header> for InfoSource {
  fn from(
    Header {
      protocol_version,
      vendor_id,
      guid_prefix,
      ..
    }: Header,
  ) -> Self {
    InfoSource {
      unused: 0,
      protocol_version,
      vendor_id,
      guid_prefix,
    }
  }
}

impl From<InfoSource> for Header {
  fn from(
    InfoSource {
      protocol_version,
      vendor_id,
      guid_prefix,
      ..
    }: InfoSource,
  ) -> Self {
    Header {
      protocol_id: ProtocolId::PROTOCOL_RTPS,
      protocol_version,
      vendor_id,
      guid_prefix,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = InfoSource,
  {
      info_source,
      InfoSource {
        unused: 0,
          protocol_version: ProtocolVersion::PROTOCOLVERSION_2_2,
          vendor_id: VendorId {
              vendor_id: [0xFF, 0xAA]
          },
          guid_prefix: GuidPrefix {
              bytes: [0x01, 0x02, 0x6D, 0x3F,
                          0x7E, 0x07, 0x00, 0x00,
                          0x01, 0x00, 0x00, 0x00]
          }
      },
      le = [0x00, 0x00, 0x00, 0x00,
            0x02, 0x02, 0xFF, 0xAA,
            0x01, 0x02, 0x6D, 0x3F,
            0x7E, 0x07, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00],
      be = [0x00, 0x00, 0x00, 0x00,
            0x02, 0x02, 0xFF, 0xAA,
            0x01, 0x02, 0x6D, 0x3F,
            0x7E, 0x07, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00]
  });
}
