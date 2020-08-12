use crate::structure::guid::GUID;
use crate::structure::sequence_number::SequenceNumber;
use crate::messages::submessages::submessage_elements::serialized_payload::SerializedPayload;
use crate::dds::ddsdata::DDSData;
use std::sync::Arc;

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Clone)]
pub enum ChangeKind {
  ALIVE,
  NOT_ALIVE_DISPOSED,
  NOT_ALIVE_UNREGISTERED,
}

#[derive(Debug, Clone)]
pub struct CacheChange {
  pub kind: ChangeKind,
  pub writer_guid: GUID,
  pub sequence_number: SequenceNumber,
  pub data_value: Option<Arc<SerializedPayload>>,
  //pub inline_qos: ParameterList,

  //stps_chage_for_reader : RTPSChangeForReader
}

impl PartialEq for CacheChange {
  fn eq(&self, other: &Self) -> bool {
    let dataeq = match &self.data_value {
      Some(d1) => match &other.data_value {
        Some(d2) => d1 == d2,
        None => false,
      },
      None => other.data_value.is_none(),
    };

    self.kind == other.kind
      && self.writer_guid == other.writer_guid
      && self.sequence_number == other.sequence_number
      && dataeq
  }
}

impl CacheChange {
  pub fn new(
    writer_guid: GUID,
    sequence_number: SequenceNumber,
    data_value: Option<DDSData>,
  ) -> CacheChange {
    /*let instance_handle;
    let data_value = match data_value {
      Some(ddsdata) => {
        instance_handle = ddsdata.instance_key.clone();
        ddsdata.value()
      }
      None => {
        instance_handle = InstanceHandle::default();
        None
      }
    };*/
    CacheChange {
      kind: ChangeKind::ALIVE,
      writer_guid,
      sequence_number,
      data_value: data_value.map( |d| d.value() ).flatten(),
      //inline_qos: ParameterList::new(),
      //rtps_chage_for_reader : RTPSChangeForReader::new(),
    }
  }
}

impl Default for CacheChange {
  fn default() -> Self {
    CacheChange::new(GUID::default(), SequenceNumber::default(), None)
  }
}
