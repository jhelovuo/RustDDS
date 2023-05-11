use speedy::{Readable, Writable};

use super::submessage_elements::crypto_footer_builtin::CryptoFooter;

#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct SecurePostfix {
  submessage_length: u16, // ushort

  crypto_footer: CryptoFooter,
}
