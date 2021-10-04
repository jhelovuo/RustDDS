use crate::dds::data_types::GUID;
use std::collections::BTreeSet;
use crate::structure::sequence_number::SequenceNumber;

#[allow(unused_imports)] use log::{debug, error, info,trace};

use std::{
  collections::{BTreeMap, HashMap, /*btree_map::Range*/ },
  cmp::max,
};

use crate::dds::{
  typedesc::TypeDesc,
  qos::{QosPolicies, QosPolicyBuilder, policy::ResourceLimits },
};
use crate::structure::time::Timestamp;

use super::{
  topic_kind::TopicKind,
  cache_change::{CacheChange},
};
use std::ops::Bound::{Included, Excluded};

/// DDSCache contains all cacheCahanges that are produced by participant or recieved by participant.
/// Each topic that is been published or been subscribed are contained in separate TopicCaches.
/// One TopicCache cotains only DDSCacheChanges of one serialized IDL datatype.
/// -> all cachechanges in same TopicCache can be serialized/deserialized same way.
/// Topic/TopicCache is identified by its name, which must be unique in the whole Domain.
#[derive(Debug)]
pub struct DDSCache {
  topic_caches: HashMap<String, TopicCache>,
}

impl DDSCache {
  pub fn new() -> DDSCache {
    DDSCache {
      topic_caches: HashMap::new(),
    }
  }

  pub fn add_new_topic(
    &mut self,
    topic_name: &str,
    topic_kind: TopicKind,
    topic_data_type: TypeDesc,
  ) {
    self.topic_caches.insert(
        topic_name.to_string(),
        TopicCache::new(topic_name, topic_kind, topic_data_type),
      );
  }

  pub fn remove_topic(&mut self, topic_name: &str) {
    if self.topic_caches.contains_key(topic_name) {
      self.topic_caches.remove(topic_name);
    }
  }

  pub fn from_topic_get_change(&self, topic_name: &str, instant: &Timestamp) 
    -> Option<&CacheChange> 
  {
    self.topic_caches.get(topic_name)
      .map( |tc| tc.get_change(instant) )
      .flatten()
  }


  /// Removes cacheChange permanently
  pub fn from_topic_remove_change(
    &mut self,
    topic_name: &str,
    instant: &Timestamp,
  ) -> Option<CacheChange> {
    match self.topic_caches.get_mut(topic_name) {
      Some(tc) => tc.remove_change(instant),
      None => {
        error!("from_topic_remove_change: Topic {:?} is not in DDSCache", topic_name); 
        None  
      }
    }
  }

  /// Removes cacheChange permanently
  pub fn from_topic_remove_before(&mut self, topic_name: &str, instant: Timestamp) 
  {
    match self.topic_caches.get_mut(topic_name) {
      Some(tc) => tc.remove_changes_before(instant),
      None => {
        error!("from_topic_remove_before: topic: {:?} is not in DDSCache", topic_name); 
      }
    }
  }


  pub fn from_topic_get_changes_in_range(
    &self,
    topic_name: &str,
    start_instant: &Timestamp,
    end_instant: &Timestamp,
  ) -> Box<dyn Iterator<Item=(Timestamp, &CacheChange)> + '_> {
    match self.topic_caches.get(topic_name) {
      Some(tc) => Box::new(tc.get_changes_in_range(start_instant, end_instant)),
      None => Box::new(vec![].into_iter()),
    }
  }

  pub fn to_topic_add_change(
    &mut self,
    topic_name: &str,
    instant: &Timestamp,
    cache_change: CacheChange,
  ) {
    match self.topic_caches.get_mut(topic_name) {
      Some(tc) => tc.add_change(instant, cache_change),
      None => {
        error!("to_topic_add_change: Topic: {:?} is not in DDSCache", topic_name); 
      }
    }
  }
}

#[derive(Debug)]
pub struct TopicCache {
  topic_name: String,
  topic_data_type: TypeDesc,
  topic_kind: TopicKind,
  topic_qos: QosPolicies,
  history_cache: DDSHistoryCache,
}

impl TopicCache {
  pub fn new(topic_name: &str, topic_kind: TopicKind, topic_data_type: TypeDesc) -> TopicCache {
    TopicCache {
      topic_name: topic_name.to_string(),
      topic_data_type,
      topic_kind,
      topic_qos: QosPolicyBuilder::new().build(),
      history_cache: DDSHistoryCache::new(),
    }
  }

  pub fn get_change(&self, instant: &Timestamp) -> Option<&CacheChange> {
    self.history_cache.get_change(instant)
  }

  pub fn add_change(&mut self, instant: &Timestamp, cache_change: CacheChange) {
    self.history_cache.add_change(instant, cache_change)
      .map( |cc_back| { 
        debug!("DDSCache insert failed topic={:?} cache_change={:?}",
                self.topic_name, cc_back); 
            }
          );
  }

  pub fn get_changes_in_range(
    &self,
    start_instant: &Timestamp,
    end_instant: &Timestamp,
  ) -> Box<dyn Iterator<Item=(Timestamp, &CacheChange)> + '_> {
    self
      .history_cache
      .get_range_of_changes(start_instant, end_instant)
  }

  ///Removes and returns value if it was found
  pub fn remove_change(&mut self, instant: &Timestamp) -> Option<CacheChange> {
    self.history_cache.remove_change(instant)
  }

  pub fn remove_changes_before(&mut self, instant: Timestamp) {
    // Look up some Topic-specific resource limit
    // and remove earliest samples until we are within limit.
    // This prevents cache from groving indefinetly.
    let max_keep_samples = self.topic_qos.resource_limits()
        .unwrap_or( ResourceLimits {
                    max_samples: 1024,
                    max_instances: 1024,
                    max_samples_per_instance: 64,
                  })
        .max_samples;
    // TODO: We cannot currently keep track of instance counts, because TopicCache or
    // DDSCache below do not know about instances.
    let remove_count = self.history_cache.changes.len() as i32 - max_keep_samples as i32;
    let split_key = 
          *self.history_cache.changes.keys()
            .take(max(0,remove_count) as usize + 1)
            .last()
            .map( |lim| max(lim,&instant) )
            .unwrap_or(&instant);
    self.history_cache.remove_changes_before(split_key)
  }

}

// This is contained in a TopicCache
#[derive(Debug)]
pub struct DDSHistoryCache {
  pub(crate) changes: BTreeMap<Timestamp, CacheChange>,
  sequence_numbers: BTreeMap<GUID,BTreeSet<SequenceNumber>>,
}

impl DDSHistoryCache {
  pub fn new() -> DDSHistoryCache {
    DDSHistoryCache {
      changes: BTreeMap::new(),
      sequence_numbers: BTreeMap::new(),
    }
  }

  fn have_sn(&self, cc: &CacheChange) -> bool {
    match self.sequence_numbers.get(&cc.writer_guid) {
      None => false,
      Some(sn_set) => sn_set.contains(&cc.sequence_number),
    }
  }

  fn insert_sn(&mut self, cc: &CacheChange) {
    self.sequence_numbers.entry(cc.writer_guid)
      .or_insert_with(|| {let mut s = BTreeSet::new(); s.insert(cc.sequence_number); s } )
      .insert(cc.sequence_number);
  }

  fn remove_sn(&mut self, cc: &CacheChange) {
    self.sequence_numbers.entry(cc.writer_guid)
      .and_modify(|s| {s.remove(&cc.sequence_number);} );

    //TODO: If this makes a SN set empty, remove it from BTreeMap.
  }

  pub fn add_change(&mut self, instant: &Timestamp, cache_change: CacheChange) -> Option<CacheChange> {
    if self.have_sn(&cache_change) {
      trace!("Received duplicate {:?} from {:?}, discarding.",
        cache_change.sequence_number, cache_change.writer_guid);
      Some(cache_change)
    } else {
      self.insert_sn(&cache_change);
      let result = self.changes.insert(*instant, cache_change);
      match result {
        None => None, // all is good. timestamp was not inserted before.
        Some(old_cc) => {
          // If this happens cahce changes were created at exactly same instant.
          error!("DDSHistoryCache already contained element with key {:?} !!!", instant);
          self.remove_sn(&old_cc);
          Some(old_cc)
        }
      }
    }
  }

  pub fn get_change(&self, instant: &Timestamp) -> Option<&CacheChange> {
    self.changes.get(instant)
  }

  pub fn get_range_of_changes(
    &self,
    start_instant: &Timestamp,
    end_instant: &Timestamp,
  ) -> Box<dyn Iterator<Item=(Timestamp, &CacheChange)> + '_> {
    Box::new(
      self
        .changes
        .range((Excluded(start_instant), Included(end_instant)))
        .map(|(i,c)| (*i,c)))
  }

  /// Removes and returns value if it was found
  pub fn remove_change(&mut self, instant: &Timestamp) -> Option<CacheChange> {
    self.changes.remove(instant)
      .map( |cc| { self.remove_sn(&cc); cc })
  }

  pub fn remove_changes_before(&mut self, instant: Timestamp) {
    let to_retain = self.changes.split_off(&instant);
    let to_remove = std::mem::replace(&mut self.changes, to_retain); 
    for r in to_remove.values() {
      self.remove_sn(r);
    } 
  }
}

// -----------------------------------------------------------------------
// -----------------------------------------------------------------------
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::sync::{Arc, RwLock};
  use std::{thread};

  use super::DDSCache;
  use crate::{
    dds::{
      data_types::DDSTimestamp, ddsdata::DDSData, data_types::DDSDuration, typedesc::TypeDesc,
    },
    messages::submessages::submessage_elements::serialized_payload::{SerializedPayload},
    structure::{
      cache_change::CacheChange, topic_kind::TopicKind, guid::GUID, sequence_number::SequenceNumber,
    },
  };

  #[test]
  fn create_dds_cache() {
    let cache = Arc::new(RwLock::new(DDSCache::new()));
    let topic_name = &String::from("ImJustATopic");
    let change1 = CacheChange::new(
      GUID::GUID_UNKNOWN,
      SequenceNumber::from(1),
      DDSData::new(SerializedPayload::default()),
    );
    cache.write().unwrap().add_new_topic(
      topic_name,
      TopicKind::WithKey,
      TypeDesc::new("IDontKnowIfThisIsNecessary"),
    );
    cache
      .write()
      .unwrap()
      .to_topic_add_change(topic_name, &DDSTimestamp::now(), change1);

    let pointerToCache1 = cache.clone();

    thread::spawn(move || {
      let topic_name = &String::from("ImJustATopic");
      let cahange2 = CacheChange::new(
        GUID::GUID_UNKNOWN,
        SequenceNumber::from(1),
        DDSData::new(SerializedPayload::default()),
      );
      pointerToCache1.write().unwrap().to_topic_add_change(
        topic_name,
        &DDSTimestamp::now(),
        cahange2,
      );
      let cahange3 = CacheChange::new(
        GUID::GUID_UNKNOWN,
        SequenceNumber::from(2),
        DDSData::new(SerializedPayload::default()),
      );
      pointerToCache1.write().unwrap().to_topic_add_change(
        topic_name,
        &DDSTimestamp::now(),
        cahange3,
      );
    })
    .join()
    .unwrap();

    cache
      .read()
      .unwrap()
      .from_topic_get_change(topic_name, &DDSTimestamp::now());
    assert_eq!(
      cache
        .read()
        .unwrap()
        .from_topic_get_changes_in_range(
          topic_name,
          &(DDSTimestamp::now() - DDSDuration::from_secs(23)),
          &DDSTimestamp::now()
        )
        .count(),
      3
    );
    // info!(
    //   "{:?}",
    //   cache.read().unwrap().from_topic_get_changes_in_range(
    //     topic_name,
    //     &(DDSTimestamp::now() - DDSDuration::from_secs(23)),
    //     &DDSTimestamp::now()
    //   )
    // );
  }
}
