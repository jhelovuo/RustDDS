use crate::structure::entity::Entity;
use crate::structure::endpoint::Endpoint;
use crate::structure::history_cache::HistoryCache;
use crate::messages::submessages::data::Data;
use crate::messages::submessages::heartbeat::Heartbeat;

#[derive(Debug, PartialEq)]
pub struct Reader {
  history_cache: HistoryCache,
} // placeholder

impl Reader {
  pub fn new() -> Reader {
    todo!()
  }

  // TODO: check if it's necessary to implement different handlers for discovery and user messages

  // handles regular data message and updates history cache
  pub fn handle_data_msg(&mut self, _msg: Data) {
    todo!()
  }

  // send ack_nack response if necessary
  pub fn handle_heartbeat(&mut self, _heartbeat: Heartbeat) {
    todo!()
  }

  // update history cache
  fn make_cache_change(&mut self) {
    todo!()
  }

  // notifies DataReaders (or any listeners that history cache has changed for this reader)
  // likely use of mio channel
  fn notify_cache_change(&self) {
    todo!()
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
