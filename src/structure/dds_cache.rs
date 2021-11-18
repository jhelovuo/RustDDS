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
    qos::{policy::ResourceLimits, QosPolicies, QosPolicyBuilder},
    typedesc::TypeDesc,
  },
  structure::{sequence_number::SequenceNumber, time::Timestamp},
};
use super::{cache_change::CacheChange, topic_kind::TopicKind};

/// DDSCache contains all cacheCahanges that are produced by participant or
/// received by participant. Each topic that is been published or been
/// subscribed are contained in separate TopicCaches. One TopicCache cotains
/// only DDSCacheChanges of one serialized IDL datatype. -> all cachechanges in
/// same TopicCache can be serialized/deserialized same way. Topic/TopicCache is
/// identified by its name, which must be unique in the whole Domain.
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
    topic_name: String,
    topic_kind: TopicKind,
    topic_data_type: TypeDesc,
  ) {
    self.topic_caches.insert(
      topic_name.clone(),
      TopicCache::new(topic_name, topic_kind, topic_data_type),
    );
  }

  pub fn remove_topic(&mut self, topic_name: &str) {
    if self.topic_caches.contains_key(topic_name) {
      self.topic_caches.remove(topic_name);
    }
  }

  pub fn from_topic_get_change(
    &self,
    topic_name: &str,
    instant: &Timestamp,
  ) -> Option<&CacheChange> {
    self
      .topic_caches
      .get(topic_name)
      .map(|tc| tc.get_change(instant))
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
        error!(
          "from_topic_remove_change: Topic {:?} is not in DDSCache",
          topic_name
        );
        None
      }
    }
  }

  /// Removes cacheChange permanently
  pub fn from_topic_remove_before(&mut self, topic_name: &str, instant: Timestamp) {
    match self.topic_caches.get_mut(topic_name) {
      Some(tc) => tc.remove_changes_before(instant),
      None => {
        error!(
          "from_topic_remove_before: topic: {:?} is not in DDSCache",
          topic_name
        );
      }
    }
  }

  pub fn from_topic_get_changes_in_range(
    &self,
    topic_name: &str,
    start_instant: &Timestamp,
    end_instant: &Timestamp,
  ) -> Box<dyn Iterator<Item = (Timestamp, &CacheChange)> + '_> {
    match self.topic_caches.get(topic_name) {
      Some(tc) => Box::new(tc.get_changes_in_range(start_instant, end_instant)),
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
pub struct TopicCache {
  topic_name: String,
  topic_data_type: TypeDesc,
  topic_kind: TopicKind,
  topic_qos: QosPolicies,
  history_cache: DDSHistoryCache,
}

impl TopicCache {
  pub fn new(topic_name: String, topic_kind: TopicKind, topic_data_type: TypeDesc) -> TopicCache {
    TopicCache {
      topic_name,
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
    self
      .history_cache
      .add_change(instant, cache_change)
      .map(|cc_back| {
        debug!(
          "DDSCache insert failed topic={:?} cache_change={:?}",
          self.topic_name, cc_back
        );
      });
  }

  pub fn get_changes_in_range(
    &self,
    start_instant: &Timestamp,
    end_instant: &Timestamp,
  ) -> Box<dyn Iterator<Item = (Timestamp, &CacheChange)> + '_> {
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
    let max_keep_samples = self
      .topic_qos
      .resource_limits()
      .unwrap_or(ResourceLimits {
        max_samples: 1024,
        max_instances: 1024,
        max_samples_per_instance: 64,
      })
      .max_samples;
    // TODO: We cannot currently keep track of instance counts, because TopicCache
    // or DDSCache below do not know about instances.
    let remove_count = self.history_cache.changes.len() as i32 - max_keep_samples as i32;
    let split_key = *self
      .history_cache
      .changes
      .keys()
      .take(max(0, remove_count) as usize + 1)
      .last()
      .map(|lim| max(lim, &instant))
      .unwrap_or(&instant);
    self.history_cache.remove_changes_before(split_key)
  }
}

// This is contained in a TopicCache
#[derive(Debug)]
pub struct DDSHistoryCache {
  pub(crate) changes: BTreeMap<Timestamp, CacheChange>,
  // sequence_numbers is an index to "changes" by GUID and SN
  sequence_numbers: BTreeMap<GUID, BTreeMap<SequenceNumber,Timestamp>>,
}

impl DDSHistoryCache {
  pub fn new() -> DDSHistoryCache {
    DDSHistoryCache {
      changes: BTreeMap::new(),
      sequence_numbers: BTreeMap::new(),
    }
  }

  fn find_by_sn(&self, cc: &CacheChange) -> Option<Timestamp> {
    self.sequence_numbers.get(&cc.writer_guid)
      .map( |snm| snm.get(&cc.sequence_number))
      .flatten()
      .cloned()
  }

  fn insert_sn(&mut self, instant: Timestamp, cc: &CacheChange) {
    self
      .sequence_numbers
      .entry(cc.writer_guid)
      .or_insert_with(|| BTreeMap::new())
      .insert(cc.sequence_number, instant);
  }

  fn remove_sn(&mut self, cc: &CacheChange) {
    let mut emptied = false;

    self.sequence_numbers.entry(cc.writer_guid)
      .and_modify(|s| { 
        s.remove(&cc.sequence_number); 
        emptied = s.is_empty(); 
      });
    if emptied { self.sequence_numbers.remove(&cc.writer_guid); } 
  }

  pub fn add_change(
    &mut self,
    instant: &Timestamp,
    cache_change: CacheChange,
  ) -> Option<CacheChange> {
    match self.find_by_sn(&cache_change) {
      Some(old_instant) => {
        // Got duplicate DATA for a SN that we already have. It should be discarded.
        //
        // Excpet that we cannot, because it is apparently legal in Discovery to keep sending with
        // the same SN the participant announcement. If we drop the duplicates here, the
        // Discovery DataReader will not get any announcements after the first one, which will
        // lead to dropping the participant then the lease time expires.
        // At least eProsma FastRTPS seems to do this.
        //
        // See https://github.com/eProsima/Fast-DDS/issues/35
        // and
        // RTPS spec 2.5 Section "8.5.3.1 General Approach":
        // "This is achieved by periodically calling
        //  StatelessWriter::unsent_changes_reset, which causes the
        //  StatelessWriter to resend all changes present in its HistoryCache
        //  to all locators."
        //
        // Reading the definition of unsent_changes_reset from "8.4.7.2.4 unsent_changes_reset",
        // looks like not incrementing SNs is conformant to the spec. 
        // OTOH, according to testing, e.g. RTI Connext does increment the SN.
        // Maybe this is a bug in the spec?
        //
        // So we update the indexing instant, and receive timestamp.
        // DataReader has to worry about not giving duplicate SN objects to the application,
        // unless they are Discovery, in which case Datareader must report updated duplicates.
        // This design idea is the best one I could think of today.

        debug!("Received duplicate {:?} from {:?}, updating timestamp.",
          cache_change.sequence_number,
          cache_change.writer_guid
        );
        let old_cc = self.remove_change(&old_instant)  // This also removes from SN index
          .expect("DDSCache index corrupted"); // we just checked it exists in the index
        // re-insert the new one.
        self.insert_sn(*instant, &cache_change);
        // TODO: Sanity check that the new CacheChange is identical to the old one.
        // If not, we are getting inconsistent retransmits.
        self.changes.insert(*instant, cache_change); // TODO: If this returns Some(_), then we have duplicate instant.
        Some(old_cc)
      }
      None => { // This is a new (to us) SequenceNumber, this is the default processing path.
        self.insert_sn( *instant, &cache_change );
        let result = self.changes.insert(*instant, cache_change);
        match result {
          None => None, // all is good. timestamp was not inserted before.
          Some(old_cc) => {
            // If this happens cahce changes were created at exactly same instant.
            error!(
              "DDSHistoryCache already contained element with key {:?} !!!",
              instant
            );
            self.remove_sn(&old_cc);
            Some(old_cc)
          }
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
  ) -> Box<dyn Iterator<Item = (Timestamp, &CacheChange)> + '_> {
    Box::new(
      self
        .changes
        .range((Excluded(start_instant), Included(end_instant)))
        .map(|(i, c)| (*i, c)),
    )
  }

  /// Removes and returns value if it was found
  pub fn remove_change(&mut self, instant: &Timestamp) -> Option<CacheChange> {
    self.changes.remove(instant).map(|cc| {
      self.remove_sn(&cc);
      cc
    })
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
  use std::{
    sync::{Arc, RwLock},
    thread,
  };

  use super::DDSCache;
  use crate::{
    dds::{
      data_types::{DDSDuration, DDSTimestamp},
      ddsdata::DDSData,
      typedesc::TypeDesc,
    },
    messages::submessages::submessage_elements::serialized_payload::SerializedPayload,
    structure::{
      cache_change::CacheChange, guid::GUID, sequence_number::SequenceNumber, topic_kind::TopicKind,
    },
  };

  #[test]
  fn create_dds_cache() {
    let cache = Arc::new(RwLock::new(DDSCache::new()));
    let topic_name = String::from("ImJustATopic");
    let change1 = CacheChange::new(
      GUID::GUID_UNKNOWN,
      SequenceNumber::from(1),
      None,
      DDSData::new(SerializedPayload::default()),
    );
    cache.write().unwrap().add_new_topic(
      topic_name.clone(),
      TopicKind::WithKey,
      TypeDesc::new("IDontKnowIfThisIsNecessary".to_string()),
    );
    cache
      .write()
      .unwrap()
      .add_change(&topic_name, &DDSTimestamp::now(), change1);

    let pointer_to_cache_1 = cache.clone();

    thread::spawn(move || {
      let topic_name = String::from("ImJustATopic");
      let cahange2 = CacheChange::new(
        GUID::GUID_UNKNOWN,
        SequenceNumber::from(2),
        None,
        DDSData::new(SerializedPayload::default()),
      );
      pointer_to_cache_1
        .write()
        .unwrap()
        .add_change(&topic_name, &DDSTimestamp::now(), cahange2);
      let cahange3 = CacheChange::new(
        GUID::GUID_UNKNOWN,
        SequenceNumber::from(3),
        None,
        DDSData::new(SerializedPayload::default()),
      );
      pointer_to_cache_1
        .write()
        .unwrap()
        .add_change(&topic_name, &DDSTimestamp::now(), cahange3);
    })
    .join()
    .unwrap();

    cache
      .read()
      .unwrap()
      .from_topic_get_change(&topic_name, &DDSTimestamp::now());
    assert_eq!(
      cache
        .read()
        .unwrap()
        .from_topic_get_changes_in_range(
          &topic_name,
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
