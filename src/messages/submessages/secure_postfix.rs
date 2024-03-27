use speedy::{Readable, Writable};
use enumflags2::BitFlags;

use super::{
  elements::crypto_footer::CryptoFooter,
  submessage::SecuritySubmessage,
  submessage_flag::{FromEndianness, SECUREPOSTFIX_Flags},
  submessage_kind::SubmessageKind,
  submessages::SubmessageHeader,
};
use crate::{
  create_security_error_and_log,
  rtps::{Submessage, SubmessageBody},
  security::{SecurityError, SecurityResult},
};

/// SecurePostfixSubMsg: section 7.3.6.5 of the Security specification (v. 1.1)
/// See section 7.3.7.7
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct SecurePostfix {
  pub(crate) crypto_footer: CryptoFooter,
}
impl SecurePostfix {
  pub fn create_submessage(self, endianness: speedy::Endianness) -> SecurityResult<Submessage> {
    let flags: BitFlags<SECUREPOSTFIX_Flags> = BitFlags::from_endianness(endianness);
    self
      .write_to_vec()
      .map(|bytes| Submessage {
        header: SubmessageHeader {
          kind: SubmessageKind::SEC_POSTFIX,
          flags: flags.bits(),
          content_length: bytes.len() as u16,
        },
        body: SubmessageBody::Security(SecuritySubmessage::SecurePostfix(self, flags)),
        original_bytes: None,
      })
      .map_err(|e| {
        create_security_error_and_log!(
          "Security plugin couldn't write SecurePostfix to bytes. Error: {}",
          e
        )
      })
  }
}
