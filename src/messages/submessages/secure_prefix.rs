use enumflags2::BitFlags;
use speedy::{Readable, Writable};

use crate::{
  rtps::{Submessage, SubmessageBody},
  security::{SecurityError, SecurityResult},
  security_error,
};
use super::{
  elements::crypto_header::CryptoHeader,
  submessage::SecuritySubmessage,
  submessage_flag::{FromEndianness, SECUREPREFIX_Flags},
  submessage_kind::SubmessageKind,
  submessages::SubmessageHeader,
};

/// SecurePrefixSubMsg: section 7.3.6.4 of the Security specification (v. 1.1)
/// See sections 7.3.7.3 and 7.3.7.6.1
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct SecurePrefix {
  pub(crate) crypto_header: CryptoHeader,
}

impl SecurePrefix {
  pub fn create_submessage(self, endianness: speedy::Endianness) -> SecurityResult<Submessage> {
    let flags: BitFlags<SECUREPREFIX_Flags> = BitFlags::from_endianness(endianness);
    self
      .write_to_vec()
      .map(|bytes| Submessage {
        header: SubmessageHeader {
          kind: SubmessageKind::SEC_PREFIX,
          flags: flags.bits(),
          content_length: bytes.len() as u16,
        },
        body: SubmessageBody::Security(SecuritySubmessage::SecurePrefix(self, flags)),
      })
      .map_err(|e| {
        security_error!(
          "Security plugin couldn't write SecurePrefix to bytes. Error: {}",
          e
        )
      })
  }
}
