use speedy::{Readable, Writable};

use super::submessage_elements::crypto_header::CryptoHeader;

/// See sections 7.3.7.3 and 7.3.7.6.1
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct SecurePrefix {
  submessage_length: u16, // ushort

  crypto_header: CryptoHeader,
}
