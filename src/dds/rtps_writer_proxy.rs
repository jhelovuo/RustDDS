use crate::structure::locator::LocatorList;
use crate::structure::guid::{EntityId, GUID};

pub struct RtpsWriterProxy {
  ///Identifies the remote matched Writer
  pub remote_writer_guid: GUID,

  /// List of unicast (address, port) combinations that can be used to send
  ///messages to the matched Writer or Writers. The list may be empty.
  pub unicast_locator_list: LocatorList,

  /// List of multicast (address, port) combinations that can be used to send
  /// messages to the matched Writer orWriters. The list may be empty.
  pub multicast_locator_list: LocatorList,

  /// List of CacheChanges received or expected from the matched RTPS Writer
  // changes_from_writer: HistryCache
  /// Identifies the group to which the matched Reader belongs
  pub remote_group_entity_id: EntityId,
}

impl RtpsWriterProxy {
  pub fn new(remote_writer_guid: GUID) -> Self {
    Self {
      remote_writer_guid,
      unicast_locator_list: LocatorList::default(),
      multicast_locator_list: LocatorList::default(),
      remote_group_entity_id: EntityId::ENTITYID_UNKNOWN,
    }
  }
}
