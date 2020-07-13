use crate::structure::entity::Entity;
use crate::structure::endpoint::{Endpoint, EndpointAttributes};
use crate::structure::history_cache::HistoryCache;
use crate::messages::submessages::data::Data;
use crate::messages::submessages::heartbeat::Heartbeat;
use crate::structure::entity::EntityAttributes;
use crate::structure::guid::GUID;

use crate::structure::cache_change::CacheChange;

#[derive(Debug, PartialEq)]
pub struct Reader {
  history_cache: HistoryCache,
  entity_attributes: EntityAttributes,
  pub enpoint_attributes: EndpointAttributes,
} // placeholder

impl Reader {
  pub fn new(guid: GUID) -> Reader {
    Reader {
      history_cache: HistoryCache::new(),
      entity_attributes: EntityAttributes{guid},
      enpoint_attributes: EndpointAttributes::default(),
    }
  }
  // TODO: check if it's necessary to implement different handlers for discovery
  // and user messages

  // handles regular data message and updates history cache
  pub fn handle_data_msg(&mut self, data: Data) { 
    let user_data = true; // Different action for discovery data?
    if user_data {
      self.make_cache_change(data);
    }else {
      // is discovery data
      todo!();
    }
  }

  // send ack_nack response if necessary
  pub fn handle_heartbeat(&mut self, _heartbeat: Heartbeat) {
    todo!()
  }  

  // update history cache
  fn make_cache_change(&mut self, data: Data) {
    let change =  CacheChange::new(
      self.get_guid(),
      data.writer_sn,
      Some(data),
    );
    self.history_cache.add_change(change);
  }

  // notifies DataReaders (or any listeners that history cache has changed for this reader)
  // likely use of mio channel
  fn notify_cache_change(&self) {
    todo!()
  }
}

impl Entity for Reader {
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
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
  use crate::structure::guid::GUID;

  use crate::dds::reader::Reader;

  #[test]
  fn test_creating_cache_changes() {
   
    let reader = Reader::new(GUID::new());

   
  }
 

}
