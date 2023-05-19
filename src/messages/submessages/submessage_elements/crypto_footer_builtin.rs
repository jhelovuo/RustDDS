use speedy::{Readable, Writable};

/// CryptoFooter: section 7.3.6.2.4 of the Security specification (v.
/// 1.1)
//Enumerate on possible values of `transformation_id`.
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub enum CryptoFooter {
  Dummy, //TODO. This variant is here just to suppress compiler warning.
}
