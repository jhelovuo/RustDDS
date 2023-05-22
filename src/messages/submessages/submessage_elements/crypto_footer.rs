use speedy::{Readable, Writable};

/// CryptoFooter: section 7.3.6.2.4 of the Security specification (v.
/// 1.1)
/// Should be interpreted by the plugin based on `transformation_id`
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct CryptoFooter {
  pub data: Vec<u8>,
}
