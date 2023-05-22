use speedy::{Readable, Writable};

use super::submessage_elements::crypto_content::CryptoContent;

#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct SecureBody {
  submessage_length: u16, // ushort

  crypto_content: CryptoContent,
}
