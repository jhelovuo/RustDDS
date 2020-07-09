use crate::structure::entity::Entity;
use crate::structure::endpoint::Endpoint;
use crate::structure::history_cache::HistoryCache;
use crate::messages::submessages::data::Data;
use crate::messages::submessages::heartbeat::Heartbeat;

use crate::serialization::cdrDeserializer;
use crate::structure::cache_change::CacheChange;
use crate::structure::cache_change::ChangeData;
use crate::structure::guid::GUID;
use crate::structure::cache_change::ChangeKind;
use crate::structure::instance_handle::InstanceHandle;
use crate::structure::sequence_number::SequenceNumber;



#[derive(Debug, PartialEq)]
pub struct Reader {
  history_cache: HistoryCache
  //expectedType: str,
} // placeholder

impl Reader {
  pub fn new() -> Reader {
    todo!()
  }


  // TODO: check if it's necessary to implement different handlers for discovery and user messages

  // handles regular data message and updates history cache
  pub fn handle_data_msg(&mut self, mut _msg: Data) { 
    
    // todo how to know expected data type?


    self.make_cache_change_for_data_message(_msg);

  }

  // send ack_nack response if necessary
  pub fn handle_heartbeat(&mut self, _heartbeat: Heartbeat) {
    todo!()
  }

  fn make_cache_change_for_data_message(&mut self, mut _msg: Data){

    let isLittleEndian : bool = Reader::test_if_little_endian(_msg.serialized_payload.representation_identifier);
      
    let deserializedMessage;
    if isLittleEndian{
      deserializedMessage = cdrDeserializer::DeserializerLittleEndian::deserialize_from_little_endian(&mut _msg.serialized_payload.value)
    }
    else{
      todo!();
    }

    let g = GUID::new();
    g.from_prefix(_msg.writer_id);
    let change =  CacheChange {
       kind : ChangeKind::ALIVE,
       writer_guid: g ,
       instance_handle: InstanceHandle::default(),
       sequence_number: SequenceNumber::default(),
       //TODO????
       //data_value: Some(ChangeData{hasLittleEndianData :true, deserializedLittleEndianData: deserializedMessage})
       data_value: Some(ChangeData{hasLittleEndianData :true})
    };

    self.history_cache.add_change(change);
  }

  // update history cache
  fn make_cache_change(&mut self, _msg: Data) {

    

    // TODO send 
    
  }

  // notifies DataReaders (or any listeners that history cache has changed for this reader)
  // likely use of mio channel
  fn notify_cache_change(&self) {
    todo!()
  }

  fn test_if_little_endian(representation_identifier :u16) -> bool{
      if representation_identifier == 0x00{
        true
      }else{
        false
      }
    }
  }


impl Entity for Reader {
  fn as_entity(&self) -> &crate::structure::entity::EntityAttributes {
    todo!()
  }
}

impl Endpoint for Reader {
  fn as_endpoint(&self) -> &crate::structure::endpoint::EndpointAttributes {
    todo!()
  }
}





#[cfg(test)]
mod tests {
  use crate::serialization::cdrSerializer::to_little_endian_binary;
  use crate::serialization::cdrDeserializer::deserialize_from_little_endian;
  use serde::{Serialize, Deserialize};

  use crate::dds::reader::Reader;

  #[test]
  fn test_creating_cache_changes() {
   
    let reader = Reader::new();

   
  }
 

}
