use speedy::{Context, Readable, Reader, Writable, Writer};

/// A SerializedPayload contains the serialized representation of
/// either value of an application-defined data-object or
/// the value of the key that uniquely identifies the data-object
#[derive(Debug, PartialEq, Readable, Writable, Clone)]
pub struct SerializedPayload {
  pub representation_identifier: u16,
  pub representation_options: u16,
  pub value: Vec<u8>,
}

impl SerializedPayload {
  pub fn new() -> SerializedPayload {
    SerializedPayload {
      representation_identifier: 0,
      representation_options: 0,
      value: vec![],
    }
  }
}
impl Default for SerializedPayload {
  fn default() -> SerializedPayload {
    SerializedPayload::new()
  }
}