use speedy::{Readable, Writable};
use enumflags2::BitFlags;

use super::{
  elements::crypto_footer::CryptoFooter,
  submessage::SecuritySubmessage,
  submessage_flag::{FromEndianness, SECURERTPSPOSTFIX_Flags},
  submessage_kind::SubmessageKind,
  submessages::SubmessageHeader,
};
use crate::{
  create_security_error_and_log,
  rtps::{Submessage, SubmessageBody},
  security::{SecurityError, SecurityResult},
};

/// SecureRTPSPostfixSubMsg: section 7.3.6.7 of the Security specification (v.
/// 1.1) See section 7.3.7.9
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct SecureRTPSPostfix {
  pub(crate) crypto_footer: CryptoFooter,
}

impl SecureRTPSPostfix {
  pub fn create_submessage(self, endianness: speedy::Endianness) -> SecurityResult<Submessage> {
    let flags: BitFlags<SECURERTPSPOSTFIX_Flags> = BitFlags::from_endianness(endianness);
    self
      .write_to_vec()
      .map(|bytes| Submessage {
        header: SubmessageHeader {
          kind: SubmessageKind::SRTPS_POSTFIX,
          flags: flags.bits(),
          content_length: bytes.len() as u16,
        },
        body: SubmessageBody::Security(SecuritySubmessage::SecureRTPSPostfix(self, flags)),
        original_bytes: None,
      })
      .map_err(|e| {
        create_security_error_and_log!(
          "Security plugin couldn't write SecureRTPSPostfix to bytes. Error: {}",
          e
        )
      })
  }
}
