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
  rtps::{Submessage, SubmessageBody},
  security::{SecurityError, SecurityResult},
  security_error,
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
      .write_to_vec()
      .map(|bytes| Submessage {
        header: SubmessageHeader {
          kind: SubmessageKind::SEC_BODY,
          flags: flags.bits(),
          content_length: bytes.len() as u16,
        },
        body: SubmessageBody::Security(SecuritySubmessage::SecureBody(self, flags)),
      })
      .map_err(|e| {
        security_error!(
          "Security plugin couldn't write SecureBody to bytes. Error: {}",
          e
        )
      })
  }
}
