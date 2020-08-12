use std::{
  time::Instant,
  collections::{BTreeMap, HashMap, btree_map::Range},
};
use crate::dds::{typedesc::TypeDesc, qos::QosPolicies};
use super::{topic_kind::TopicKind, cache_change::{ChangeKind, CacheChange}};
use std::ops::Bound::Included;

///DDSCache contains all cacheCahanges that are produced by participant or recieved by participant.
///Each topic that is been published or been subscribed are contained in separate TopicCaches.
///One TopicCache cotains only DDSCacheChanges of one serialized IDL datatype.
///-> all cachechanges in same TopicCache can be serialized/deserialized same way.
///Topic/TopicCache is identified by its name, which must be unique in the whole Domain.
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
    topic_name: &String,
    topic_kind: TopicKind,
    topic_data_type: &TypeDesc,
  ) -> bool {
    if self.topic_caches.contains_key(topic_name) {
      print!("Topic with same name already added to DDSCache!");
      return false;
    } else {
      self.topic_caches.insert(
        topic_name.to_string(),
        TopicCache::new(topic_kind, topic_data_type.clone()),
      );
      return true;
    }
  }

  pub fn remove_topic(&mut self, topic_name: &String) {
    if self.topic_caches.contains_key(topic_name) {
      self.topic_caches.remove(topic_name);
    }
  }

  pub fn get_topic_qos_mut(&mut self, topic_name: &String) -> Option<&mut QosPolicies> {
    if self.topic_caches.contains_key(topic_name) {
      return Some(&mut self.topic_caches.get_mut(topic_name).unwrap().topic_qos);
    } else {
      return None;
    }
  }

  pub fn get_topic_qos(&self, topic_name: &String) -> Option<&QosPolicies> {
    if self.topic_caches.contains_key(topic_name) {
      return Some(&self.topic_caches.get(topic_name).unwrap().topic_qos);
    } else {
      return None;
    }
  }

  pub fn from_topic_get_change(
    &self,
    topic_name: &String,
    instant: &Instant,
  ) -> Option<&CacheChange> {
    if self.topic_caches.contains_key(topic_name) {
      return self
        .topic_caches
        .get(topic_name)
        .unwrap()
        .get_change(instant);
    } else {
      panic!("Topic: '{:?}' is not in DDSCache", topic_name);
    }
  }

  /// Sets cacheChange to not alive disposed. So its waiting to be permanently removed.
  pub fn from_topic_set_change_to_not_alive_disposed(&mut self, topic_name : &String, instant : &Instant){
    if self.topic_caches.contains_key(topic_name) {
      self.topic_caches.get_mut(topic_name).unwrap().set_change_to_not_alive_disposed(instant);
    }else{
      panic!("Topic: '{:?}' is not in DDSCache",topic_name);
    }
  }

  /// Removes cacheChange permanently
  pub fn from_topic_remove_change(&mut self, topic_name : &String, instant : &Instant) -> Option<CacheChange>{
    if self.topic_caches.contains_key(topic_name) {
      return self
        .topic_caches
        .get_mut(topic_name)
        .unwrap()
        .remove_change(instant);
    } else {
      panic!("Topic: '{:?}' is not in DDSCache", topic_name);
    }
  }

  pub fn from_topic_get_changes_in_range(
    &self,
    topic_name: &String,
    start_instant: &Instant,
    end_instant: &Instant,
  ) -> Vec<(&Instant, &CacheChange)> {
    if self.topic_caches.contains_key(topic_name) {
      return self
        .topic_caches
        .get(topic_name)
        .unwrap()
        .get_changes_in_range(start_instant, end_instant);
    } else {
      return vec![];
    }
  }

  pub fn to_topic_add_change(
    &mut self,
    topic_name: &String,
    instant: &Instant,
    cache_change: CacheChange,
  ) {
    if self.topic_caches.contains_key(topic_name) {
      return self
        .topic_caches
        .get_mut(topic_name)
        .unwrap()
        .add_change(instant, cache_change);
    } else {
      panic!("Topic: '{:?}' is not added to DDSCache", topic_name);
    }
  }
}

#[derive(Debug)]
pub struct TopicCache {
  topic_data_type: TypeDesc,
  topic_kind: TopicKind,
  topic_qos: QosPolicies,
  history_cache: DDSHistoryCache,
}

impl TopicCache {
  pub fn new(topic_kind: TopicKind, topic_data_type: TypeDesc) -> TopicCache {
    TopicCache {
      topic_data_type: topic_data_type,
      topic_kind: topic_kind,
      topic_qos: QosPolicies::qos_none(),
      history_cache: DDSHistoryCache::new(),
    }
  }
  pub fn get_change(&self, instant: &Instant) -> Option<&CacheChange> {
    self.history_cache.get_change(instant)
  }

  pub fn add_change(&mut self, instant: &Instant, cache_change: CacheChange) {
    self.history_cache.add_change(instant, cache_change)
  }

  pub fn get_changes_in_range(
    &self,
    start_instant: &Instant,
    end_instant: &Instant,
  ) -> Vec<(&Instant, &CacheChange)> {
    self
      .history_cache
      .get_range_of_changes_vec(start_instant, end_instant)
  }

  ///Removes and returns value if it was found
  pub fn remove_change(&mut self, instant: &Instant) -> Option<CacheChange> {
    return self.history_cache.remove_change(instant);
  }

  pub fn set_change_to_not_alive_disposed(&mut self, instant : &Instant){
    self.history_cache.change_change_kind(instant, ChangeKind::NOT_ALIVE_DISPOSED);
  }
}

#[derive(Debug)]
pub struct DDSHistoryCache {
  changes: BTreeMap<Instant, CacheChange>,
}

impl DDSHistoryCache {
  pub fn new() -> DDSHistoryCache {
    DDSHistoryCache {
      changes: BTreeMap::new(),
    }
  }

  pub fn add_change(&mut self, instant: &Instant, cache_change: CacheChange) {
    let result = self.changes.insert(*instant, cache_change);
    if result.is_none() {
      // all is good. timestamp was not inserted before.
    }
    else{
      // If this happens cahce changes were created at exactly same instant.
      panic!("DDSHistoryCache already contained element with key !!!");
    }
  }

  pub fn get_change(&self, instant: &Instant) -> Option<&CacheChange> {
    self.changes.get(instant)
  }

  pub fn get_range_of_changes(
    &self,
    start_instant: &Instant,
    end_instant: &Instant,
  ) -> Range<Instant, CacheChange> {
    self
      .changes
      .range((Included(start_instant), Included(end_instant)))
  }

  pub fn get_range_of_changes_vec(
    &self,
    start_instant: &Instant,
    end_instant: &Instant,
  ) -> Vec<(&Instant, &CacheChange)> {
    let mut changes: Vec<(&Instant, &CacheChange)> = vec![];
    for (i, c) in self
      .changes
      .range((Included(start_instant), Included(end_instant)))
    {
      changes.push((i, c));
    }
    return changes;
  }
  
  pub fn change_change_kind(&mut self, instant : &Instant, change_kind : ChangeKind){
    let change = self.changes.get_mut(instant);
    if change.is_some(){
      change.unwrap().kind = change_kind;
    }
    else{
      panic!("CacheChange with instance: {:?} was not found on DDSHistoryCache!", instant);
    }
  }

  /*
  /// returns element with LARGEST timestamp
  pub fn get_latest_change(&self) -> Option<&CacheChange>{
    if  self.changes.last_entry().is_none(){
      return None;
    }
    else{
      let key_to_change = self.changes.last_entry().unwrap().key();
      return self.changes.get(key_to_change);
    }
  }
  */

  /// Removes and returns value if it was found
  pub fn remove_change(&mut self, instant: &Instant) -> Option<CacheChange> {
    self.changes.remove(instant)
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, RwLock};
  use std::{
    time::{Duration, Instant},
    thread,
  };
  use super::DDSCache;
  use crate::{
    dds::{typedesc::TypeDesc, ddsdata::DDSData},
    structure::{
      cache_change::CacheChange, topic_kind::TopicKind, guid::GUID, sequence_number::SequenceNumber,
    },
    messages::submessages::submessage_elements::serialized_payload::SerializedPayload,
  };

  #[test]
  fn create_dds_cache() {
    let cache = Arc::new(RwLock::new(DDSCache::new()));
    let topic_name = &String::from("ImJustATopic");
    let change1 = CacheChange::new(
      GUID::GUID_UNKNOWN,
      SequenceNumber::from(1),
      Some(DDSData::new(SerializedPayload::new())),
    );
    cache.write().unwrap().add_new_topic(
      topic_name,
      TopicKind::WITH_KEY,
      &TypeDesc::new("IDontKnowIfThisIsNecessary".to_string()),
    );
    cache
      .write()
      .unwrap()
      .to_topic_add_change(topic_name, &Instant::now(), change1);

    let pointerToCache1 = cache.clone();

    thread::spawn(move || {
      let topic_name = &String::from("ImJustATopic");
      let cahange2 = CacheChange::new(
        GUID::GUID_UNKNOWN,
        SequenceNumber::from(1),
        Some(DDSData::new(SerializedPayload::new())),
      );
      pointerToCache1
        .write()
        .unwrap()
        .to_topic_add_change(topic_name, &Instant::now(), cahange2);
      let cahange3 = CacheChange::new(
        GUID::GUID_UNKNOWN,
        SequenceNumber::from(2),
        Some(DDSData::new(SerializedPayload::new())),
      );
      pointerToCache1
        .write()
        .unwrap()
        .to_topic_add_change(topic_name, &Instant::now(), cahange3);
    })
    .join()
    .unwrap();

    cache
      .read()
      .unwrap()
      .from_topic_get_change(topic_name, &Instant::now());
    assert_eq!(
      cache
        .read()
        .unwrap()
        .from_topic_get_changes_in_range(
          topic_name,
          &(Instant::now() - Duration::from_secs(23)),
          &Instant::now()
        )
        .len(),
      3
    );
    println!(
      "{:?}",
      cache.read().unwrap().from_topic_get_changes_in_range(
        topic_name,
        &(Instant::now() - Duration::from_secs(23)),
        &Instant::now()
      )
    );
  }
}
