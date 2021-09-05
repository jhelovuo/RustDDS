use crate::{
  serialization::SubMessage, serialization::SubmessageBody, structure::guid::GuidPrefix,
  messages::submessages::submessages::SubmessageHeader,
};
use enumflags2::BitFlags;
use log::error;
use speedy::{Readable, Writable};

use super::{
  submessage::InterpreterSubmessage, submessage_flag::INFODESTINATION_Flags,
  submessage_kind::SubmessageKind,
};

/// This message is sent from an RTPS Writer to an RTPS Reader
/// to modify the GuidPrefix used to interpret the Reader entityIds
/// appearing in the Submessages that follow it.
#[derive(Debug, PartialEq, Clone, Readable, Writable)]
pub struct InfoDestination {
  /// Provides the GuidPrefix that should be used to reconstruct the GUIDs
  /// of all the RTPS Reader entities whose EntityIds appears
  /// in the Submessages that follow.
  pub guid_prefix: GuidPrefix,
}

impl InfoDestination {
  pub fn create_submessage(self, flags: BitFlags<INFODESTINATION_Flags>) -> Option<SubMessage> {
    let submessage_len = match self.write_to_vec() {
      Ok(bytes) => bytes.len() as u16,
      Err(e) => {
        error!(
          "Reader couldn't write info destination to bytes. Error: {}",
          e
        );
        return None;
      }
    };

    let infodst_header = SubmessageHeader {
      kind: SubmessageKind::INFO_DST,
      flags: flags.bits(),
      content_length: submessage_len,
    };

    Some(SubMessage {
      header: infodst_header,
      body: SubmessageBody::Interpreter(InterpreterSubmessage::InfoDestination(self, flags)),
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = InfoDestination,
  {
      info_destination,
      InfoDestination {
          guid_prefix: GuidPrefix {
              entityKey: [0x01, 0x02, 0x6D, 0x3F,
                          0x7E, 0x07, 0x00, 0x00,
                          0x01, 0x00, 0x00, 0x00]
          }
      },
      le = [0x01, 0x02, 0x6D, 0x3F,
            0x7E, 0x07, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00],
      be = [0x01, 0x02, 0x6D, 0x3F,
            0x7E, 0x07, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00]
  });
}
