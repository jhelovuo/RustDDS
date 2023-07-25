use speedy::{Readable, Writable};
use enumflags2::BitFlags;

use super::{
  elements::crypto_header::CryptoHeader,
  submessage::SecuritySubmessage,
  submessage_flag::{FromEndianness, SECURERTPSPREFIX_Flags},
  submessage_kind::SubmessageKind,
  submessages::SubmessageHeader,
};
use crate::{
  rtps::{Submessage, SubmessageBody},
  security::{SecurityError, SecurityResult},
  security_error,
};

/// SecureRTPSPrefixSubMsg: section 7.3.6.6 of the Security specification (v.
/// 1.1) See sections 7.3.7.3 and 7.3.7.8.1
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct SecureRTPSPrefix {
  pub(crate) crypto_header: CryptoHeader,
}

impl SecureRTPSPrefix {
  pub fn create_submessage(self, endianness: speedy::Endianness) -> SecurityResult<Submessage> {
    let flags: BitFlags<SECURERTPSPREFIX_Flags> = BitFlags::from_endianness(endianness);
    self
      .write_to_vec()
      .map(|bytes| Submessage {
        header: SubmessageHeader {
          kind: SubmessageKind::SRTPS_PREFIX,
          flags: flags.bits(),
          content_length: bytes.len() as u16,
        },
        body: SubmessageBody::Security(SecuritySubmessage::SecureRTPSPrefix(self, flags)),
      })
      .map_err(|e| {
        security_error!(
          "Security plugin couldn't write SecureRTPSPrefix to bytes. Error: {}",
          e
        )
      })
  }
}
