use std::{
  collections::{BTreeMap, BTreeSet, HashMap, },
  ops::Bound,
  //ops::Deref,
  //cmp::max,
};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

//use serde::de::DeserializeOwned;

use crate::{
  dds::{
    //ddsdata::DDSData,
    qos::{policy, QosPolicies},
    readcondition::ReadCondition,
    sampleinfo::*,
    //topic::{Topic, TopicDescription,},
    traits::key::{Key, Keyed, },
    //traits::serde_adapters::with_key::*,
    with_key::datasample::DataSample,
  },
  structure::{
    cache_change::DeserializedCacheChange, 
    guid::GUID,
    sequence_number::SequenceNumber,
    time::Timestamp,
    //dds_cache::DDSCache,
  },
  with_key::WriteOptions,
};

//use std::num::Zero; unstable

// DataSampleCache is a structure local to DataReader and DataWriter. It acts as
// a buffer between e.g. RTPS Reader and the application-facing DataReader. It
// keeps track of what each DataReader has "read" or "taken".


// Data samples are here ordered and indexed by Timestamp, which must be a
// unique key. RTPS Timestamp has sub-nanosecond resolution, so it could be
// unique, provided that the source clock ticks frequently enough.
pub struct DataSampleCache<D: Keyed> {
  qos: QosPolicies,
  datasamples: BTreeMap<Timestamp, SampleWithMetaData<D>>, /* ordered storage for deserialized
                                                            * samples */
  pub(crate) instance_map: BTreeMap<D::K, InstanceMetaData>, // ordered storage for instances
  //hash_to_key_map: BTreeMap<KeyHash, D::K>,

  //latest_instant: Timestamp, // how far we already have samples.
  //latest_sequence_number: BTreeMap<GUID, SequenceNumber>, // latest SN pointer for each writer in our Topic
}

pub(crate) struct InstanceMetaData {
  instance_samples: BTreeSet<Timestamp>, // which samples belong to this instance
  instance_state: InstanceState,         // latest known alive/not_alive state for this instance
  latest_generation_available: NotAliveGenerationCounts, // in this instance
  last_generation_accessed: NotAliveGenerationCounts, // in this instance
}

struct SampleWithMetaData<D: Keyed> {
  // a snapshot of the instance-wide counts
  // at the time this sample was received.
  generation_counts: NotAliveGenerationCounts,
  writer_guid: GUID,               // who wrote this
  sequence_number: SequenceNumber, // as sent by the Writer
  write_options: WriteOptions,     // as stamped by Writer
  sample_has_been_read: bool,      // sample_state

  // the data sample (or key) itself is stored here
  sample: Result<D, D::K>,
}

impl<D> SampleWithMetaData<D>
where
  D: Keyed,
  <D as Keyed>::K: Key,
{
  pub fn key(&self) -> D::K {
    match &self.sample {
      Ok(d) => d.key(),
      Err(k) => k.clone(),
    }
  }
}

impl<D> DataSampleCache<D>
where
  D: Keyed,
  <D as Keyed>::K: Key,
{
  pub fn new(qos: QosPolicies) -> Self {
    Self {
      qos,
      datasamples: BTreeMap::new(),
      instance_map: BTreeMap::new(),
      //hash_to_key_map: BTreeMap::new(),
      //latest_instant: Timestamp::ZERO, 
      //latest_sequence_number: BTreeMap::new(),
    }
  }


  pub(crate) fn fill_from_deserialized_cache_change(&mut self, deserialized_cc : DeserializedCacheChange<D>)
  {
    // TODO list.


    self.add_sample(
      deserialized_cc.sample,
      deserialized_cc.writer_guid,
      deserialized_cc.sequence_number,
      deserialized_cc.receive_instant,
      deserialized_cc.write_options);
  }

  /*
  pub fn fill_from_dds_cache<DA>(&mut self, is_reliable: bool, dds_cache:impl Deref<Target=DDSCache>, my_topic:Topic)
  where
    DA: DeserializerAdapter<D>,
    D: DeserializeOwned,
  {

    let cache_changes = dds_cache.topic_get_changes_in_range(
      &my_topic.name(),
      &self.latest_instant,
      &Timestamp::now(),
    );

    let mut cache_changes_vec: Vec<(Timestamp, &CacheChange)> = cache_changes.collect();

    // We sort by sequence number so that earlier SNs (from the same writer) are
    // forced to appear earlier. This way we do not lose any CacheChanges even if
    // they were received out of order.
    // The next loop will discard any CacheChanges that appear out of sequence.

    if cache_changes_vec.len() > 1 &&
      // Check if the sequence is already sorted, as we expect this to be really common.
      ! cache_changes_vec.iter().zip(cache_changes_vec.iter().skip(1))
        .all(|((_,a),(_,b))| a.sequence_number <= b.sequence_number)
      // TODO: We can change this to .is_sorted_by_key( ) when it lands in stable std library.
    {  
      cache_changes_vec.sort_by_key(|(_ts, cc)| cc.sequence_number);
      // .sort_by_key function documentation says it is optimized for almost-sorted case,
      // which is good here.
    }

    for (
      instant,
      CacheChange {
        writer_guid,
        sequence_number,
        write_options,
        data_value,
      },
    ) in cache_changes_vec
    {
      self.latest_instant = max(self.latest_instant, instant); // update our time pointer
                                                               // what was the latest
      let latest_sequence_number_have_already = self.latest_sequence_number.get(writer_guid);

      // Check that the sequence numbers proceed in correct order.
      // The reliable mode gets stuck if DDSCache is not able to produce
      // all the SNs withot gaps. (TODO: Ensure that DDSCache satisfies this.)
      //
      // TODO: This does not work if there are gaps in the SequenceNumber sequence.
      // This can happen if the Writer at the other end of the wire informs our Reader
      // that some SNs are not available, either by a Heartbeat or a Gap submessage.
      // 
      // The current implementation may get stuck if there is a gap in a reliable topic. 
      //
      // If no previous SN is known, then any SN is acceptable, as we may be
      // joining the data stream at any time.
      //
      if (! is_reliable &&
            // Check that SequenceNumber always goes forward.
            // Getting the same SN means duplicate packet, which we must drop.
            latest_sequence_number_have_already
              .map_or(true,  |latest| sequence_number > latest))
        || (is_reliable &&
            // Check that we get all the sequence numbers in order
            latest_sequence_number_have_already
              .map_or(true, |latest| *latest + SequenceNumber::from(1) == *sequence_number))
      {
        // normal case: sequence_number not seen before
        // first, update our last-seen-pointer
        self
          .latest_sequence_number
          .insert(*writer_guid, *sequence_number);

        // deserialize into datasample cache
        match data_value {
          DDSData::Data { serialized_payload } => {
            // what is our data serialization format (representation identifier) ?
            if let Some(recognized_rep_id) = DA::supported_encodings()
              .iter()
              .find(|r| **r == serialized_payload.representation_identifier)
            {
              match DA::from_bytes(&serialized_payload.value, *recognized_rep_id) {
                Ok(payload) => self.add_sample(
                  Ok(payload),
                  *writer_guid,
                  *sequence_number,
                  instant,
                  write_options.clone(),
                ),
                Err(e) => {
                  error!(
                    "Failed to deserialize bytes: {}, Topic = {}, Type = {:?}",
                    e,
                    my_topic.name(),
                    my_topic.get_type()
                  );
                  info!("Bytes were {:?}", &serialized_payload.value);
                  continue; // skip this sample
                }
              }
            } else {
              warn!(
                "Unknown representation id {:?}.",
                serialized_payload.representation_identifier
              );
              info!("Serialized payload was {:?}", &serialized_payload);
              continue; // skip this sample, as we cannot decode it
            }
          }

          DDSData::DisposeByKey {
            key: serialized_key,
            ..
          } => {
            match DA::key_from_bytes(
              &serialized_key.value,
              serialized_key.representation_identifier,
            ) {
              Ok(key) => {
                self.add_sample(
                  Err(key),
                  *writer_guid,
                  *sequence_number,
                  instant,
                  write_options.clone(),
                );
              }
              Err(e) => {
                warn!(
                  "Failed to deserialize key {}, Topic = {}, Type = {:?}",
                  e,
                  my_topic.name(),
                  my_topic.get_type()
                );
                debug!("Bytes were {:?}", &serialized_key.value);
                continue; // skip this sample
              }
            }
          }

          DDSData::DisposeByKeyHash { key_hash, .. } => {
            if let Some(key) = self.key_by_hash(*key_hash) {
              self.add_sample(
                Err(key),
                *writer_guid,
                *sequence_number,
                instant,
                write_options.clone(),
              );
            } else {
              warn!("Tried to dispose with unkonwn key hash: {:x?}", key_hash);
              // The cache should know hash -> key mapping even if the sample
              // has been disposed or .take()n
            }
          } 
        } // match
      }
      // if (acceptable SN)
      else {
        // sequence naumber is not acceptable
      }
    } // for loop

  }
*/
  fn add_sample(
    &mut self,
    new_sample: Result<D, D::K>,
    writer_guid: GUID,
    sequence_number: SequenceNumber,
    receive_timestamp: Timestamp,
    write_options: WriteOptions,
  ) {
    let instance_key = match &new_sample {
      Ok(d) => d.key(),
      Err(k) => k.clone(),
    };

    let new_instance_state = match new_sample {
      Ok(_) => InstanceState::Alive,
      Err(_) => InstanceState::NotAliveDisposed,
    };

    // find or create metadata record
    let instance_metadata = if let Some(imd) = self.instance_map.get_mut(&instance_key) {
      imd
    } else {
      // not found, create new one.
      let imd = InstanceMetaData {
        instance_samples: BTreeSet::new(),
        instance_state: new_instance_state,
        latest_generation_available: NotAliveGenerationCounts::zero(), /* this is new instance,
                                                                        * so start from zero */
        last_generation_accessed: NotAliveGenerationCounts::sub_zero(), // never accessed
      };
      self.instance_map.insert(instance_key.clone(), imd);
      // self
      //   .hash_to_key_map
      //   .insert(instance_key.hash_key(), instance_key.clone());
      self
        .instance_map
        .get_mut(&instance_key)
        // must succeed, since this was just inserted
        .unwrap()
    };

    // update instance metadata
    instance_metadata.instance_samples.insert(receive_timestamp);

    match (instance_metadata.instance_state, new_instance_state) {
      (InstanceState::Alive, _) => (), // was Alive, does not change counts

      (InstanceState::NotAliveDisposed, InstanceState::Alive) =>
      // born again
      {
        instance_metadata
          .latest_generation_available
          .disposed_generation_count += 1;
      }

      (InstanceState::NotAliveDisposed, _) => (), // you can only die once

      (InstanceState::NotAliveNoWriters, InstanceState::Alive) =>
      // born again
      {
        instance_metadata
          .latest_generation_available
          .no_writers_generation_count += 1;
      }

      (InstanceState::NotAliveNoWriters, _) => (), // you can only die once
    }
    instance_metadata.instance_state = new_instance_state;

    // insert new_sample to main table
    self
      .datasamples
      .insert(
        receive_timestamp,
        SampleWithMetaData {
          generation_counts: instance_metadata.latest_generation_available,
          writer_guid,
          sequence_number,
          write_options,
          sample_has_been_read: false,
          sample: new_sample,
        },
      )
      .map_or_else(
        /* None: ok */ || (),
        /* Some: key was there already! */
        // TODO: We should not outright panic here, but rather raise a serious error.
        // This is a symption that the receive timestamps are not unique identifiers like they are supposed to be.
        |_already_existed| {
          panic!(
            "Tried to add duplicate datasample with the same key {:?}",
            receive_timestamp
          );
        },
      );

    // garbage collect
    let sample_keep_history_limit: Option<i32> = match self.qos.history() {
      Some(policy::History::KeepAll) => None, // no limit
      Some(policy::History::KeepLast { depth }) => Some(depth),
      None => Some(1), // default history policy
    };
    let sample_keep_resource_limit = if let Some(policy::ResourceLimits {
      max_samples: _,
      max_instances: _,
      max_samples_per_instance,
    }) = self.qos.resource_limits
    {
      Some(max_samples_per_instance)
    } else {
      None
    };

    if let Some(instance_keep_count) = sample_keep_history_limit.or(sample_keep_resource_limit) {
      let remove_count = instance_metadata.instance_samples.len() as i32 - instance_keep_count;
      if remove_count > 0 {
        let keys_to_remove: Vec<_> = instance_metadata
          .instance_samples
          .iter()
          .take(remove_count as usize)
          .copied()
          .collect();
        for k in keys_to_remove {
          instance_metadata.instance_samples.remove(&k);
          self.datasamples.remove(&k);
        }
      }
    }

    // TODO: Implement other resource_limit settings than max_instances_per
    // sample, i.e.
  }

  // Calling select_(instance)_keys_for access does not constitute access, i.e.
  // it does not change any state of the cache.
  // Samples are marked read or viewed only when "read" or "take" methods (below)
  // are called.
  pub fn select_keys_for_access(&self, rc: ReadCondition) -> Vec<(Timestamp, D::K)> {
    self
      .datasamples
      .iter()
      .filter_map(|(ts, dsm)| {
        let key = dsm.key();
        if self.sample_selector(&rc, self.instance_map.get(&key).unwrap(), dsm) {
          Some((*ts, key))
        } else {
          None
        }
      })
      .collect()
  }

  pub fn select_instance_keys_for_access(
    &self,
    instance: &D::K,
    rc: ReadCondition,
  ) -> Vec<(Timestamp, D::K)> {
    match self.instance_map.get(instance) {
      None => Vec::new(),
      Some(imd) => imd
        .instance_samples
        .iter()
        .filter_map(|ts| {
          if let Some(ds) = self.datasamples.get(ts) {
            if self.sample_selector(&rc, imd, ds) {
              Some((*ts, instance.clone()))
            } else {
              None
            }
          } else {
            None
          }
        })
        .collect(),
    }
  }

  // select helper
  fn sample_selector(
    &self,
    rc: &ReadCondition,
    imd: &InstanceMetaData,
    d: &SampleWithMetaData<D>,
  ) -> bool {
    // check sample state
    (*rc.sample_state_mask() == SampleState::any()
      || rc.sample_state_mask()
          .contains( if d.sample_has_been_read { SampleState::Read } else {SampleState::NotRead} ) )
    &&
    // check view state
    (*rc.view_state_mask() == ViewState::any()
      ||
      { let sample_gen = d.generation_counts.total();
        let last_accessed = imd.last_generation_accessed.total();
        let is_new = sample_gen > last_accessed;
        rc.view_state_mask()
          .contains( if is_new { ViewState::New} else { ViewState::NotNew }  )
      }
    )
    &&
    // check instance state
    (*rc.instance_state_mask() == InstanceState::any()
      || rc.instance_state_mask()
          .contains( imd.instance_state )
    )
  }

  fn make_sample_info(
    dswm: &SampleWithMetaData<D>,
    imd: &InstanceMetaData,
    sample_rank: usize,
    mrs_generations: i32,
    mrsic_generations: i32,
  ) -> SampleInfo {
    SampleInfo {
      sample_state: if dswm.sample_has_been_read {
        SampleState::Read
      } else {
        SampleState::NotRead
      },
      view_state: if dswm.generation_counts.total() > imd.last_generation_accessed.total() {
        ViewState::New
      } else {
        ViewState::NotNew
      },
      instance_state: imd.instance_state,
      generation_counts: dswm.generation_counts,
      sample_rank: sample_rank as i32, // how many samples follow this one
      generation_rank: mrsic_generations - dswm.generation_counts.total(),
      absolute_generation_rank: mrs_generations - dswm.generation_counts.total(),
      write_options: dswm.write_options.clone(),
      publication_handle: dswm.writer_guid,
      sequence_number: dswm.sequence_number,
    }
  }

  fn record_instance_generation_viewed(
    instance_generations: &mut HashMap<D::K, NotAliveGenerationCounts>,
    accessed_generations: NotAliveGenerationCounts,
    instance_key: &D::K,
  ) {
    instance_generations
      .entry(instance_key.clone())
      .and_modify(|old_gens| {
        if accessed_generations.total() > old_gens.total() {
          *old_gens = accessed_generations;
        }
      })
      .or_insert(accessed_generations);
  }

  fn mark_instances_viewed(
    &mut self,
    instance_generations: &HashMap<D::K, NotAliveGenerationCounts>,
  ) {
    for (inst, gen) in instance_generations {
      if let Some(imd) = self.instance_map.get_mut(inst) {
        imd.last_generation_accessed = *gen;
      } else {
        panic!("Instance disappeared!?!!1!");
      }
    }
  }

  // read methods perform actual read or take. They must be called with key
  // vectors obtained from select_*_for_access -methods above, or their
  // subvectors.
  //
  // Therea are two versions of both read and take: Return DataSample<D> (incl.
  // metadata) and "bare" versions without metadata.
  /*
  pub fn read_by_keys(&mut self, keys: &[(Timestamp, D::K)]) -> Vec<DataSample<&D>> {
    let len = keys.len();
    let mut result = Vec::with_capacity(len);

    if len == 0 {
      return result;
    }

    let mut instance_generations: HashMap<D::K, NotAliveGenerationCounts> = HashMap::new();
    let mrsic_total = self
      .instance_map
      .get(&keys.last().unwrap().1)
      .unwrap()
      .latest_generation_available
      .total();
    let mrs_total = self
      .datasamples
      .iter()
      .next_back()
      .unwrap()
      .1
      .generation_counts
      .total();
    let mut sample_infos = VecDeque::with_capacity(len);
    // construct SampleInfos and record read/viewed
    for (index, (ts, key)) in keys.iter().enumerate() {
      let dswm = self.datasamples.get_mut(ts).unwrap();
      let imd = self.instance_map.get(key).unwrap();

      let sample_info = Self::make_sample_info(dswm, imd, len - index - 1, mrs_total, mrsic_total);
      dswm.sample_has_been_read = true; // mark as read
      Self::record_instance_generation_viewed(
        &mut instance_generations,
        dswm.generation_counts,
        key,
      );
      sample_infos.push_back(sample_info);
    }

    // mark instances viewed
    self.mark_instances_viewed(&instance_generations);

    // We need to do SampleInfo construction and final result construction as
    // separate passes. This is becaue SampleInfo construction needs to mark
    // items as read and generations as viewed, i.e. needs mutable reference to
    // data_samples. Result construction (in read, not take) needs to hand out
    // multiple references into data_samples, therefore it needs immutable
    // access, not mutable.

    // construct results
    for (ts, _key) in keys.iter() {
      let sample_info = sample_infos.pop_front().unwrap();
      let sample: &std::result::Result<D, D::K> = &self.datasamples.get(ts).unwrap().sample;
      result.push(DataSample::new(
        sample_info,
        result_ok_as_ref_err_clone(sample),
      ));
    }

    result
  }
  */
  pub fn take_by_keys(&mut self, keys: &[(Timestamp, D::K)]) -> Vec<DataSample<D>> {
    let len = keys.len();
    let mut result = Vec::with_capacity(len);

    if len == 0 {
      return result;
    }

    let mut instance_generations: HashMap<D::K, NotAliveGenerationCounts> = HashMap::new();
    let mrsic_total = self
      .instance_map
      .get(&keys.last().unwrap().1)
      .unwrap()
      .latest_generation_available
      .total();
    let mrs_total = self
      .datasamples
      .iter()
      .next_back()
      .unwrap()
      .1
      .generation_counts
      .total();
    // collect result
    for (index, (ts, key)) in keys.iter().enumerate() {
      let dswm = self.datasamples.remove(ts).unwrap();
      let imd = self.instance_map.get(key).unwrap();
      let sample_info = Self::make_sample_info(&dswm, imd, len - index - 1, mrs_total, mrsic_total);
      //dwsm.sample_has_been_read = true; // no need to mark read, as the dswm is
      // about to be destroyed
      Self::record_instance_generation_viewed(
        &mut instance_generations,
        dswm.generation_counts,
        key,
      );
      result.push(DataSample::new(sample_info, dswm.sample));
    }

    self.mark_instances_viewed(&instance_generations);
    result
  }
  /*
  pub fn read_bare_by_keys(
    &mut self,
    keys: &[(Timestamp, D::K)],
  ) -> Vec<std::result::Result<&D, D::K>> {
    let len = keys.len();
    let mut result = Vec::with_capacity(len);

    if len == 0 {
      return result;
    }

    let mut instance_generations: HashMap<D::K, NotAliveGenerationCounts> = HashMap::new();

    // construct SampleInfos and record read/viewed
    for (ts, key) in keys.iter() {
      let dswm = self.datasamples.get_mut(ts).unwrap();
      dswm.sample_has_been_read = true; // mark as read
      Self::record_instance_generation_viewed(
        &mut instance_generations,
        dswm.generation_counts,
        key,
      );
    }

    self.mark_instances_viewed(&instance_generations);

    // We need to do SampleInfo construction and final result construction as
    // separate passes. See reason in read function above.

    // construct results
    for (ts, _key) in keys.iter() {
      result.push(result_ok_as_ref_err_clone(
        &self.datasamples.get(ts).unwrap().sample,
      ));
    }
    result
  }
  */
  pub fn take_bare_by_keys(
    &mut self,
    keys: &[(Timestamp, D::K)],
  ) -> Vec<std::result::Result<D, D::K>> {
    let len = keys.len();
    let mut result = Vec::with_capacity(len);

    if len == 0 {
      return result;
    }

    let mut instance_generations: HashMap<D::K, NotAliveGenerationCounts> = HashMap::new();

    for (ts, key) in keys.iter() {
      let dswm = self.datasamples.remove(ts).unwrap();
      //dwsm.sample_has_been_read = true; // no need to mark read, as the dswm is
      // about to be destroyed
      Self::record_instance_generation_viewed(
        &mut instance_generations,
        dswm.generation_counts,
        key,
      );
      result.push(dswm.sample);
    }

    self.mark_instances_viewed(&instance_generations);
    result
  }

  // pub fn key_by_hash(&self, key_hash: KeyHash) -> Option<D::K> {
  //   if let Some(k) = self.hash_to_key_map.get(&key_hash) {
  //     Some(k.clone())
  //   } else {
  //     debug!(
  //       "key_by_hash: requested KeyHash {:?}, but not found. I have {:?}",
  //       key_hash,
  //       self.hash_to_key_map.keys()
  //     );
  //     None
  //   }
  // }

  pub fn next_key(&self, key: &D::K) -> Option<D::K> {
    self
      .instance_map
      .range((Bound::Excluded(key), Bound::Unbounded))
      .map(|(k, _)| k.clone())
      .next()
  }

  // This is currently unused, as HasQosPolicy trait does not allow setting QoS,
  // but a placeholder if it is ever implemented.
  // #[allow(dead_code)]
  // pub fn set_qos_policy(&mut self, qos: QosPolicies) {
  //   self.qos = qos;
  // }
}

#[cfg(test)]
mod tests {
  // use super::*;
  // use crate::{
  //   structure::{time::Timestamp},
  // };
  // use crate::dds::ddsdata::DDSData;
  // use crate::dds::traits::key::Keyed;
  // use crate::test::random_data::*;

  #[test]
  fn dsc_empty_qos() {
    /*
    let qos = QosPolicies::qos_none();
    let mut datasample_cache = DataSampleCache::<RandomData>::new(qos);

    let timestamp = Timestamp::now();
    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    let org_ddsdata = DDSData::from(&data, Some(timestamp));

    let key = data.key().clone();
    datasample_cache.add_sample(Ok(data.clone()), GUID::GUID_UNKNOWN, timestamp, None);
    //datasample_cache.add_datasample(datasample).unwrap();

    let samples = datasample_cache.read_by_keys(&[(timestamp, key)]);
    assert_eq!(samples.len(), 1);
    match &samples.get(0).unwrap().value() {
      Ok(huh) => {
        let ddssample = DDSData::from(huh, Some(timestamp));
        assert_eq!(org_ddsdata, ddssample);
      }
      _ => (),
    }
    */
  }
}
