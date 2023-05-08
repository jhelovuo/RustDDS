/// TODO: CryptoToken: section 8.5.1.1 of the Security specification (v. 1.1)
pub struct CryptoToken {}

/// TODO: ParticipantCryptoHandle: section 8.5.1.2 of the Security specification (v. 1.1)
pub struct ParticipantCryptoHandle {}

/// TODO: DatawriterCryptoHandle: section 8.5.1.3 of the Security specification (v. 1.1)
pub struct DatawriterCryptoHandle {}

/// TODO: DatareaderCryptoHandle: section 8.5.1.4 of the Security specification (v. 1.1)
pub struct DatareaderCryptoHandle {}

/// CryptoTransformIdentifier: section 8.5.1.5 of the Security specification (v. 1.1)
type CryptoTransformKind = [u8; 4];
type CryptoTransformKeyId = [u8; 4];
pub struct CryptoTransformIdentifier {
  pub transformation_kind: CryptoTransformKind,
  pub transformation_key_id: CryptoTransformKeyId,
}

/// SecureSubmessageCategory_t: section 8.5.1.6 of the Security specification (v. 1.1)
pub enum SecureSubmessageCategory {
    InfoSubmessage,
    DatawriterSubmessage,
    DatareaderSubmessage
}
