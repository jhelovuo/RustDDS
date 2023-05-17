use speedy::{Readable, Writable};

use crate::security::cryptographic::types::CryptoTransformIdentifier;

/// CryptoHeader: section 7.3.6.2.3 of the Security specification (v.
/// 1.1)
/// See section 7.3.7.3
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct CryptoHeader {
  transformation_id: CryptoTransformIdentifier,
  plugin_crypto_header_extra: PluginCryptoHeaderExtra,
}

//Enumerate on possible values of `transformation_id`.
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub enum PluginCryptoHeaderExtra {
  Dummy, //TODO. This variant is here just to suppress compiler warning.
}
