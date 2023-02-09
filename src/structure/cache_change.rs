use crate::{
  dds::{ddsdata::DDSData, with_key::datawriter::WriteOptions},
  dds::traits::key::*,
  structure::{guid::GUID, sequence_number::SequenceNumber, time::Timestamp,},
};

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
  pub write_options: WriteOptions,
  pub data_value: DDSData,
}

#[cfg(test)]
impl PartialEq for CacheChange {
  fn eq(&self, other: &Self) -> bool {
    self.writer_guid == other.writer_guid
      && self.sequence_number == other.sequence_number
      && self.write_options == other.write_options
      && self.data_value == other.data_value
  }
}

impl CacheChange {
  pub fn new(
    writer_guid: GUID,
    sequence_number: SequenceNumber,
    write_options: WriteOptions,
    data_value: DDSData,
  ) -> Self {
    Self {
      writer_guid,
      sequence_number,
      write_options,
      data_value,
    }
  }

  // Not needed?
  // pub fn change_kind(&self) -> ChangeKind {
  //   self.data_value.change_kind()
  // }
}


// This structure is used to communicate just deserialized samples
// from SimpleDatareader to DataReader
#[derive(Debug, Clone)]
pub(crate) struct DeserializedCacheChange<D: Keyed> {
  pub(crate) receive_instant: Timestamp,  // to be used as unique key in internal data structures
  pub(crate) writer_guid: GUID,
  pub(crate) sequence_number: SequenceNumber,
  pub(crate) write_options: WriteOptions,

  // the data sample (or key) itself is stored here
  pub(crate) sample: Result<D, D::K>,  // TODO: make this a Box<> for easier detaching an reattaching to somewhere else
}

impl<D:Keyed> DeserializedCacheChange<D> {
  pub fn new(receive_instant: Timestamp, cc: &CacheChange, deserialized: Result<D,D::K>, ) -> Self {
    DeserializedCacheChange {
      receive_instant,
      writer_guid: cc.writer_guid,
      sequence_number: cc.sequence_number,
      write_options: cc.write_options.clone(),
      sample: deserialized,
    }
  }
}