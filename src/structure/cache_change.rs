use crate::structure::guid::GUID;
use crate::structure::instance_handle::InstanceHandle;
use crate::structure::sequence_number::SequenceNumber;
use crate::serialization::cdrDeserializer::DeserializerLittleEndian;

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub enum ChangeKind {
  ALIVE,
  NOT_ALIVE_DISPOSED,
  NOT_ALIVE_UNREGISTERED,
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct ChangeData {
  //serializedMessage : Vec<u8>,
  pub hasLittleEndianData: bool,
  //pub deserializedLittleEndianData : DeserializerLittleEndian,
}


/*
#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct Data {
  hasDeserializedData : bool,
  serializedMessage : Vec<u8>,
  //deserializedMessage : DeserializedMessageContainer,


}

impl Data {
  pub fn new() -> Data{
    Data { serializedMessage : Vec::new(),
           hasDeserializedData :false}
  }
}
//pub struct DeserializedMessageContainer<T>{
//  data : T,
//}

*/
#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct CacheChange {
  pub kind: ChangeKind,
  pub writer_guid: GUID,
  pub instance_handle: InstanceHandle,
  pub sequence_number: SequenceNumber,
  pub data_value: Option<ChangeData>,
}
