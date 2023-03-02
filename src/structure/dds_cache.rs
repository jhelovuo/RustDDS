use std::{
  cmp::max,
  collections::{BTreeMap, HashMap},
  ops::Bound::{Excluded, Included},
};

#[allow(unused_imports)]
use log::{debug, error, info, trace};

use crate::{
  dds::{
    data_types::GUID,
    qos::{policy::ResourceLimits, policy::History, QosPolicies, },
    typedesc::TypeDesc,
  },
  structure::{sequence_number::SequenceNumber, time::Timestamp},
};
use super::cache_change::CacheChange;

/// DDSCache contains all cacheCahanges that are produced by participant or
/// received by participant. Each topic that is been published or been
/// subscribed are contained in separate TopicCaches. One TopicCache contains
/// only DDSCacheChanges of one serialized IDL datatype. -> all cachechanges in
/// same TopicCache can be serialized/deserialized same way. Topic/TopicCache is
/// identified by its name, which must be unique in the whole Domain.
#[derive(Debug, Default)]
pub struct DDSCache {
  topic_caches: HashMap<String, TopicCache>,
}

impl DDSCache {
  pub fn new() -> Self {
    Self::default()
  }
  pub fn mark_reliably_received_before(&mut self, topic_name: &str, writer: GUID, sn:SequenceNumber) {
    self.topic_caches.get_mut(topic_name)
      .and_then(|tc| Some(tc.mark_reliably_received_before(writer,sn)) );
  }
  // Insert new topic if it does not exist.
  // If it exists already, update cache size limits.
  // TODO: If we pick up a topic from Discovery, can someone DoS us by
  // sending super large limits in Topic QoS?
  pub fn add_new_topic(&mut self, topic_name: String, topic_data_type: TypeDesc, qos: &QosPolicies) {
    self
      .topic_caches
      .entry(topic_name.clone())
      .and_modify(|t| t.update_keep_limits(qos) )
      .or_insert_with(|| TopicCache::new(topic_name, topic_data_type, qos));
  }

  // TODO: Investigate why this is not used.
  // When do RTPS Topics die? Never?
  #[allow(dead_code)]
  pub fn remove_topic(&mut self, topic_name: &str) {
    if self.topic_caches.contains_key(topic_name) {
      self.topic_caches.remove(topic_name);
    }
  }

  pub fn topic_get_change(&self, topic_name: &str, instant: &Timestamp) -> Option<&CacheChange> {
    self
      .topic_caches
      .get(topic_name)
      .and_then(|tc| tc.get_change(instant))
  }


  /// Removes cacheChange permanently
  pub fn topic_remove_before(&mut self, topic_name: &str, instant: Timestamp) {
    match self.topic_caches.get_mut(topic_name) {
      Some(tc) => tc.remove_changes_before(instant),
      None => {
        error!(
          "topic_remove_before: topic: {:?} is not in DDSCache",
          topic_name
        );
      }
    }
  }

  pub fn get_changes_in_range_best_effort(
    &self,
    topic_name: &str,
    start_instant: Timestamp,
    end_instant: Timestamp,
  ) -> Box<dyn Iterator<Item = (Timestamp, &CacheChange)> + '_> {
    match self.topic_caches.get(topic_name) {
      Some(tc) => Box::new(tc.get_changes_in_range_best_effort(start_instant, end_instant)),
      None => Box::new(vec![].into_iter()),
    }
  }

  pub fn get_changes_in_range_reliable<'a>(
    &'a self,
    topic_name: &str,
    last_read_sn: &'a BTreeMap<GUID,SequenceNumber>,
  ) -> Box<dyn Iterator<Item = (Timestamp, &CacheChange)> + 'a> {
    match self.topic_caches.get(topic_name) {
      Some(tc) => Box::new(tc.get_changes_in_range_reliable(last_read_sn)),
      None => Box::new(vec![].into_iter()),
    }
  }

  pub fn add_change(&mut self, topic_name: &str, instant: &Timestamp, cache_change: CacheChange) {
    match self.topic_caches.get_mut(topic_name) {
      Some(tc) => tc.add_change(instant, cache_change),
      None => {
        error!(
          "to_topic_add_change: Topic: {:?} is not in DDSCache",
          topic_name
        );
      }
    }
  }
}


#[derive(Debug)]
struct TopicCache {
  topic_name: String,
  #[allow(dead_code)] // TODO: Which (future) feature needs this?
  topic_data_type: TypeDesc,
  #[allow(dead_code)] // TODO: The relevant data here is in min/max keep_samples. Is this still relevant?
  topic_qos: QosPolicies,
  min_keep_samples: History, 
  max_keep_samples: i32, // from QoS, for quick, repeated access
  //TODO: Change this to Option<u32>, where None means "no limit". 

  // Tha main content of the cache is in this map.
  // Timestamp is assumed to be unique id over all the ChacheChanges.
  changes: BTreeMap<Timestamp, CacheChange>,

  // sequence_numbers is an index to "changes" by GUID and SN
  sequence_numbers: BTreeMap<GUID, BTreeMap<SequenceNumber, Timestamp>>,

  // Keep track of how far we have "reliably" received samples from each Writer
  // This means that all data up to this point has either been received, or
  // we have been notified (GAP or HEARTBEAT) that is not available and never will.
  // Therefore, data before the marker SN can be handed off to a Reliable DataReader.
  // Initially, we consider the marker for each Writer (GUID) to be SequenceNumber::new(1)
  received_reliably_before: BTreeMap<GUID, SequenceNumber>

}

impl TopicCache {
  pub fn new(topic_name: String, topic_data_type: TypeDesc, topic_qos: &QosPolicies) -> Self {
    let mut  new_self = Self {
      topic_name,
      topic_data_type,
      topic_qos: topic_qos.clone(),
      min_keep_samples: History::KeepLast{ depth: 1 }, // dummy value, next call will overwrite this
      max_keep_samples: 1, // dummy value, next call will overwrite this
      changes: BTreeMap::new(),
      sequence_numbers: BTreeMap::new(),
      received_reliably_before: BTreeMap::new(),
    };

    new_self.update_keep_limits(topic_qos);

    new_self
  }

  fn update_keep_limits(&mut self, qos: &QosPolicies) {
    let min_keep_samples = qos.history()
      // default history setting from DDS spec v1.4 Section 2.2.3 "Supported QoS",
      // Table at p.99
      .unwrap_or( History::KeepLast{ depth: 1 } );

    // Look up some Topic-specific resource limit
    // and remove earliest samples until we are within limit.
    // This prevents cache from groving indefinetly.
    let max_keep_samples = qos
      .resource_limits()
      .unwrap_or(ResourceLimits {
        max_samples: 1024,
        max_instances: 1024,
        max_samples_per_instance: 64,
      })
      .max_samples;
    // TODO: We cannot currently keep track of instance counts, because TopicCache
    // or DDSCache below do not know about instances.


    // If a definite minimum is apecified, increase resource limit to at least that.
    let max_keep_samples =
      match min_keep_samples {
        History::KeepLast{depth: n} if n > max_keep_samples => n,
        _ => max_keep_samples
      };

    // actual update. This is will only ever increase cache size.
    self.min_keep_samples = max(min_keep_samples , self.min_keep_samples);
    self.max_keep_samples = max(max_keep_samples , self.max_keep_samples);
  }

  pub fn mark_reliably_received_before(&mut self, writer: GUID, sn:SequenceNumber) {
    self.received_reliably_before.insert(writer,sn);
  }

  pub fn get_change(&self, instant: &Timestamp) -> Option<&CacheChange> {
    self.changes.get(instant)
  }

  pub fn add_change(&mut self, instant: &Timestamp, cache_change: CacheChange) {
    self
      .add_change_internal(instant, cache_change)
      .map(|cc_back| {
        debug!(
          "DDSCache insert failed topic={:?} cache_change={:?}",
          self.topic_name, cc_back
        );
      });
  }

  fn add_change_internal(
    &mut self,
    instant: &Timestamp,
    cache_change: CacheChange,
  ) -> Option<CacheChange> {
    // First, do garbage collection.
    // But not at everey insert, just to save time and effort.
    // Some heuristic to decide if we should collect now.
    let payload_size = max(1, cache_change.data_value.payload_size());
    let semi_random_number = i64::from(cache_change.sequence_number) as usize;
    let fairly_large_constant = 0xffff;
    let modulus = fairly_large_constant / payload_size;
    if modulus == 0 || semi_random_number % modulus == 0 {
      debug!("Garbage collecting topic {}", self.topic_name);
      self.remove_changes_before(Timestamp::ZERO);
      // remove limit is so low that it has no effect.
      // We actually only remove to keep cache size between min and max limits.
    }

    // Now to the actual adding business.
    if let Some(old_instant) = self.find_by_sn(&cache_change) {
      // Got duplicate DATA for a SN that we already have. It should be discarded.
      debug!(
        "add_change: discarding duplicate {:?} from {:?}. old timestamp = {:?}, new = {:?}",
        cache_change.sequence_number, cache_change.writer_guid, old_instant, instant,
      );
      Some(cache_change)
    } else {
      // This is a new (to us) SequenceNumber, this is the default processing path.
      self.insert_sn(*instant, &cache_change);
      self.changes.insert(*instant, cache_change).map(|old_cc| {
        // If this happens, cache changes were created at exactly same instant.
        // This is bad, since we are using instants as keys and assume that they
        // are unique.
        error!(
          "DDSHistoryCache already contained element with key {:?} !!!",
          instant
        );
        self.remove_sn(&old_cc);
        old_cc
      })
    }
  }

  fn find_by_sn(&self, cc: &CacheChange) -> Option<Timestamp> {
    self
      .sequence_numbers
      .get(&cc.writer_guid)
      .and_then(|snm| snm.get(&cc.sequence_number))
      .copied()
  }

  fn insert_sn(&mut self, instant: Timestamp, cc: &CacheChange) {
    self
      .sequence_numbers
      .entry(cc.writer_guid)
      .or_insert_with(BTreeMap::new)
      .insert(cc.sequence_number, instant);
  }

  pub fn get_changes_in_range_best_effort(
    &self,
    start_instant: Timestamp,
    end_instant: Timestamp,
  ) -> Box<dyn Iterator<Item = (Timestamp, &CacheChange)> + '_> {
    Box::new(
      self
        .changes
        .range((Excluded(start_instant), Included(end_instant)))
        // .filter(move |(_,cc)| ! limit_by_reliability || cc.sequence_number < self.reliable_before(cc.writer_guid) )
        .map(|(i, c)| (*i, c))
    )
  }

  pub fn get_changes_in_range_reliable<'a>(
    &'a self,
    last_read_sn: &'a BTreeMap<GUID,SequenceNumber>,
  ) -> Box<dyn Iterator<Item = (Timestamp, &CacheChange)> + 'a > {
    Box::new(
      self
        .sequence_numbers
        .iter()
        .flat_map(|(guid, sn_map)| 
          {
            let lower_bound_exc = last_read_sn.get(&guid).cloned().unwrap_or(SequenceNumber::zero());
            let upper_bound_exc = self.reliable_before(*guid);
            sn_map.range(( Excluded(lower_bound_exc), Excluded(upper_bound_exc) ))
          }) // we get iterator of Timestamp
        .filter_map(|(_sn,t)| self.get_change(t).map(|cc| (*t,cc) ) )
    )
  }

  fn reliable_before(&self, writer:GUID) -> SequenceNumber {
    self.received_reliably_before
      .get(&writer)
      .cloned()
      .unwrap_or( SequenceNumber::default() )
      // Sequence numbering starts at default(), so anything before that is always received
      // reliably, since no such samples exist.
  }

  
  fn remove_sn(&mut self, cc: &CacheChange) {
    let mut emptied = false;

    self.sequence_numbers.entry(cc.writer_guid).and_modify(|s| {
      s.remove(&cc.sequence_number);
      emptied = s.is_empty();
    });
    if emptied {
      self.sequence_numbers.remove(&cc.writer_guid);
    }
  }

  /// remove changes before given Timestamp, but keep at least
  /// min_keep_samples.
  /// We must always keep below max_keep_samples.
  fn remove_changes_before(&mut self, remove_before: Timestamp) {
    let min_remove_count = 
      max(0, self.changes.len().wrapping_sub(self.max_keep_samples as usize) );

    let max_remove_count = match self.min_keep_samples {
      History::KeepLast{depth} => 
        max(min_remove_count, self.changes.len().wrapping_sub(depth as usize) ),
      History::KeepAll => min_remove_count,
    };

    // Find the first key that is to be retained, i.e. enumerate
    // one past the items to be removed.
    let split_key = *self
      .changes
      .keys()
      .take(max_remove_count)
      .enumerate()
      .skip_while(|(i,ts)| *i < min_remove_count || (**ts < remove_before && *i < max_remove_count))
      .map(|(_,ts)| ts) // un-enumerate
      .next() // the next element would be the first to retain
      .unwrap_or( &Timestamp::ZERO ); // if there is no next, then everything from ZERO onwards

    // split_off: Returns everything after the given key, including the key.
    let to_retain = self.changes.split_off(&split_key);
    let to_remove = std::mem::replace(&mut self.changes, to_retain);

    // update also SequeceNumber map
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
  use std::{
    sync::{Arc, RwLock},
    thread,
  };

  use super::DDSCache;
  use crate::{
    dds::{ddsdata::DDSData, typedesc::TypeDesc, with_key::datawriter::WriteOptions},
    messages::submessages::submessage_elements::serialized_payload::SerializedPayload,
    structure::{cache_change::CacheChange, guid::GUID, sequence_number::SequenceNumber},
  };

  #[test]
  fn create_dds_cache() {
    let cache = Arc::new(RwLock::new(DDSCache::new()));
    let topic_name = String::from("ImJustATopic");
    let change1 = CacheChange::new(
      GUID::GUID_UNKNOWN,
      SequenceNumber::new(1),
      WriteOptions::default(),
      DDSData::new(SerializedPayload::default()),
    );
    cache.write().unwrap().add_new_topic(
      topic_name.clone(),
      TypeDesc::new("IDontKnowIfThisIsNecessary".to_string()),
    );
    cache
      .write()
      .unwrap()
      .add_change(&topic_name, &crate::Timestamp::now(), change1);

    let pointer_to_cache_1 = cache.clone();

    thread::spawn(move || {
      let topic_name = String::from("ImJustATopic");
      let cahange2 = CacheChange::new(
        GUID::GUID_UNKNOWN,
        SequenceNumber::new(2),
        WriteOptions::default(),
        DDSData::new(SerializedPayload::default()),
      );
      pointer_to_cache_1.write().unwrap().add_change(
        &topic_name,
        &crate::Timestamp::now(),
        cahange2,
      );
      let cahange3 = CacheChange::new(
        GUID::GUID_UNKNOWN,
        SequenceNumber::new(3),
        WriteOptions::default(),
        DDSData::new(SerializedPayload::default()),
      );
      pointer_to_cache_1.write().unwrap().add_change(
        &topic_name,
        &crate::Timestamp::now(),
        cahange3,
      );
    })
    .join()
    .unwrap();

    cache
      .read()
      .unwrap()
      .topic_get_change(&topic_name, &crate::Timestamp::now());
    assert_eq!(
      cache
        .read()
        .unwrap()
        .topic_get_changes_in_range(
          &topic_name,
          &(crate::Timestamp::now() - crate::Duration::from_secs(23)),
          &crate::Timestamp::now()
        )
        .count(),
      3
    );
    // info!(
    //   "{:?}",
    //   cache.read().unwrap().topic_get_changes_in_range(
    //     topic_name,
    //     &(DDSTimestamp::now() - rustdds::Duration::from_secs(23)),
    //     &crate::Timestamp::now()
    //   )
    // );
  }
}
