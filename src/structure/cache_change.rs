use crate::structure::guid::GUID;
use crate::structure::sequence_number::SequenceNumber;
//use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::dds::ddsdata::DDSData;

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Copy, Clone)]
pub enum ChangeKind {
  Alive,
  NotAliveDisposed,
  NotAliveUnregistered,
}

#[derive(Debug, Clone)]
pub struct CacheChange {
  pub writer_guid: GUID,
  pub sequence_number: SequenceNumber,
  pub data_value: DDSData, 
}

#[cfg(test)]
impl PartialEq for CacheChange {
  fn eq(&self, other: &Self) -> bool {
      self.writer_guid == other.writer_guid
      && self.sequence_number == other.sequence_number
      && self.data_value == other.data_value
  }
}

impl CacheChange {
  pub fn new(
    writer_guid: GUID,
    sequence_number: SequenceNumber,
    data_value: DDSData,  
  ) -> CacheChange {
    CacheChange { writer_guid, sequence_number, data_value, }
  }

  pub fn change_kind(&self) -> ChangeKind {
    self.data_value.change_kind()
  }
}
