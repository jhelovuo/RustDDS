use speedy::{Readable, Writable};
use enumflags2::BitFlags;

use super::{
  elements::crypto_content::CryptoContent,
  submessage::SecuritySubmessage,
  submessage_flag::{FromEndianness, SECUREBODY_Flags},
  submessage_kind::SubmessageKind,
  submessages::SubmessageHeader,
};
use crate::{
  create_security_error_and_log,
  rtps::{Submessage, SubmessageBody},
  security::{SecurityError, SecurityResult},
};

/// SecureBodySubMsg: section 7.3.6.3 of the Security specification (v. 1.1)
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct SecureBody {
  pub crypto_content: CryptoContent,
}

impl SecureBody {
  pub fn create_submessage(self, endianness: speedy::Endianness) -> SecurityResult<Submessage> {
    let flags: BitFlags<SECUREBODY_Flags> = BitFlags::from_endianness(endianness);
    self
      .write_to_vec_with_ctx(endianness)
      .map(|bytes| Submessage {
        header: SubmessageHeader {
          kind: SubmessageKind::SEC_BODY,
          flags: flags.bits(),
          content_length: bytes.len() as u16,
        },
        body: SubmessageBody::Security(SecuritySubmessage::SecureBody(self, flags)),
        original_bytes: None,
      })
      .map_err(|e| {
        create_security_error_and_log!(
          "Security plugin couldn't write SecureBody to bytes. Error: {}",
          e
        )
      })
  }
}
