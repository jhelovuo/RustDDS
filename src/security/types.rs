// Property_t type from section 7.2.1 of the Security specification (v. 1.1)
pub struct Property {
  name: String,
  value: String,
  propagate: bool,
}

// BinaryProperty_t type from section 7.2.2 of the Security specification (v.
// 1.1)
pub struct BinaryProperty {
  name: String,
  value: Vec<u8>,
  propagate: bool,
}

// DataHolder type from section 7.2.3 of the Security specification (v. 1.1)
pub struct DataHolder {
  class_id: String,
  properties: Vec<Property>,
  binary_properties: Vec<BinaryProperty>,
}

// Token type from section 7.2.4 of the Security specification (v. 1.1)
pub type Token = DataHolder;

// Result type with generic OK type. Error type is SecurityError.
pub type SecurityResult<T> = std::result::Result<T, SecurityError>;

// Something like the SecurityException of the specification
#[derive(Debug, thiserror::Error)]
#[error("Security exception: {msg}")]
pub struct SecurityError {
  msg: String,
}
