/// A SerializedPayload contains the serialized representation of
/// either value of an application-defined data-object or
/// the value of the key that uniquely identifies the data-object
#[derive(Debug, PartialEq)]
pub struct SerializedPayload {
    pub value: Vec<u8>,
}
