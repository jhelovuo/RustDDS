use log::debug;

use crate::structure::{ time::Timestamp, guid::GUID };

use crate::{
  dds::traits::key::{Key, Keyed},
};


use crate::dds::with_key::datasample::DataSample;
use crate::dds::sampleinfo::*;
use crate::dds::qos::QosPolicies;
use crate::dds::qos::policy;
use crate::dds::readcondition::ReadCondition;

use std::collections::{BTreeMap, BTreeSet, HashSet, HashMap,VecDeque};
use std::ops::Bound::*;

//use std::num::Zero; unstable

// DataSampleCache is a structure local to DataReader and DataWriter. It acts as a buffer
// between e.g. RTPS Reader and the application-facing DataReader. It keeps track of what each
// DataReader has "read" or "taken".

/*
pub struct DataSampleCache<D: Keyed> {
  qos: QosPolicies,
  pub datasamples: BTreeMap<D::K, Vec<DataSample<D>>>,
  distinct_keys: HashSet<D::K>,
}
*/

// helper function
// somewhat like result.as_ref() , but one-sided only
pub(crate) fn result_ok_as_ref_err_clone<T, E: Clone>(
  r: &std::result::Result<T, E>,
) -> std::result::Result<&T, E> {
  match *r {
    Ok(ref x) => Ok(x),
    Err(ref x) => Err(x.clone()),
  }
}


// Data samples are here ordered and indexed by Timetamp, which must be a unque key.
// RTPS Timestamp has sub-nanosecond resolution, so it could be unique, provided that the source
// clock ticks frequently enough.
pub struct DataSampleCache<D: Keyed> {
  qos: QosPolicies,
  datasamples: BTreeMap<Timestamp, SampleWithMetaData<D>>,  // ordered storage for deserialized samples
  pub(crate) instance_map: BTreeMap<D::K,InstanceMetaData>, // ordered storage for instances
  hash_to_key_map: BTreeMap<u128, D::K>,
}

pub(crate) struct InstanceMetaData {
  instance_samples: BTreeSet<Timestamp>,  // which samples belong to this instance
  instance_state: InstanceState, // latest known alive/not_alive state for this instance
  latest_generation_available: NotAliveGenerationCounts, // in this instance
  last_generation_accessed: NotAliveGenerationCounts, // in this instance
}

struct SampleWithMetaData<D: Keyed> {
  // a snapshot of the instance-wide counts 
  // at the time this sample was received.
  generation_counts: NotAliveGenerationCounts,
  // who wrote this
  writer_guid: GUID,
  // timestamps
  source_timestamp: Option<Timestamp>, // as stamped by sender
  sample_has_been_read: bool, // sample_state

  // the data sample (or key) itself is stored here
  sample: Result<D, D::K>,
}

impl<D> SampleWithMetaData<D> 
where
  D: Keyed,
  <D as Keyed>::K: Key,
{
  pub fn get_key(&self) -> D::K {
    match &self.sample {
      Ok(d) => d.get_key(),
      Err(k) => k.clone(),
    }
  }
}

impl<D> DataSampleCache<D>
where
  D: Keyed,
  <D as Keyed>::K: Key,
{
  pub fn new(qos: QosPolicies) -> DataSampleCache<D> {
    DataSampleCache {
      qos,
      datasamples: BTreeMap::new(),
      instance_map: BTreeMap::new(),
      hash_to_key_map: BTreeMap::new(), 
    }
  }

  pub fn add_sample(&mut self, new_sample: Result<D, D::K>, writer_guid: GUID, 
                    receive_timestamp: Timestamp, source_timestamp: Option<Timestamp> ) {
    let instance_key = match &new_sample {
      Ok(d) => d.get_key(),
      Err(k) => k.clone(),
    };

    let new_instance_state =
      match new_sample {
        Ok(_) => InstanceState::Alive,
        Err(_) => InstanceState::NotAlive_Disposed,
      };

    // find or create metadata record
    let instance_metadata = match self.instance_map.get_mut(&instance_key) {
      // cannot use unwrap_or_else here, because of multiple borrowing.
      Some(imd) => imd,
      None => { // not found, create new one.
        let imd = InstanceMetaData { 
            instance_samples: BTreeSet::new(),
            instance_state: new_instance_state, 
            latest_generation_available: NotAliveGenerationCounts::zero(), // this is new instance, so start from zero
            last_generation_accessed: NotAliveGenerationCounts::sub_zero(), // never accessed
          }; 
        self.instance_map.insert( instance_key.clone(), imd);
        self.hash_to_key_map.insert( instance_key.into_hash_key() ,  instance_key.clone());
        self.instance_map.get_mut(&instance_key).unwrap() // must succeed, since this was just inserted
      }
    };

    // update instance metadata
    instance_metadata.instance_samples.insert(receive_timestamp.clone());

    match (instance_metadata.instance_state , new_instance_state) {
      (InstanceState::Alive , _ )=> (), // was Alive, does not change counts

      (InstanceState::NotAlive_Disposed , InstanceState::Alive ) =>
        // born again 
        instance_metadata.latest_generation_available.disposed_generation_count += 1 ,

      (InstanceState::NotAlive_Disposed , _ ) => (),  // you can only die once

      (InstanceState::NotAlive_NoWriters , InstanceState::Alive ) => 
        // born again
        instance_metadata.latest_generation_available.no_writers_generation_count += 1 ,

      (InstanceState::NotAlive_NoWriters , _ ) => (),  // you can only die once

    }
    instance_metadata.instance_state = new_instance_state;

    // insert new_sample to main table
    self.datasamples.insert( receive_timestamp, SampleWithMetaData {
      generation_counts: instance_metadata.latest_generation_available,
      writer_guid,
      source_timestamp,
      sample_has_been_read: false,
      sample: new_sample
    }).map_or_else( 
        /* None: ok*/ || () ,
        /* Some: key was there already!*/ 
        |_already_existed| panic!("Tried to add duplicate datasample with the same key {:?}", receive_timestamp)
      );

    // garbage collect
    let sample_keep_history_limit : Option<i32> = 
      match self.qos.history() {
        Some(policy::History::KeepAll) => None, // no limit
        Some(policy::History::KeepLast { depth }) => Some(*depth), 
        None => Some(1), // default history policy
      };
    let sample_keep_resource_limit = 
      if let Some(policy::ResourceLimits {max_samples, max_instances, max_samples_per_instance}) 
        = self.qos.resource_limits {
          Some(max_samples_per_instance)
        } else {None};

    if let Some(instance_keep_count) = sample_keep_history_limit.or( sample_keep_resource_limit ) {
      let remove_count = instance_metadata.instance_samples.len() as i32 - instance_keep_count;
      if remove_count > 0 {
        let keys_to_remove :Vec<_> = 
          instance_metadata.instance_samples.iter()
            .take(remove_count as usize)
            .cloned()
            .collect();
        for k in keys_to_remove {
          instance_metadata.instance_samples.remove(&k);
          self.datasamples.remove(&k);
        }
      }
    }

    // TODO: Implement other resource_limit settings than max_instances_per sample, i.e.
  }

  // Calling select_(instance)_keys_for access does not constitute access, i.e.
  // it does not change any state of the cache.
  // Samples are marked read or viewed only when "read" or "take" methods (below) are called.
  pub fn select_keys_for_access(&self, rc: ReadCondition) 
  -> Vec<(Timestamp,D::K)> {
    self.datasamples.iter()
      .filter_map( |(ts,dsm)| {
            let key = dsm.get_key();
            if self.sample_selector(&rc, self.instance_map.get(&key).unwrap(), &dsm)
              { Some((ts.clone(),key.clone())) } else { None } 
          }
        )
      .collect()
  }

  pub fn select_instance_keys_for_access(&self, instance: D::K, rc: ReadCondition) 
  -> Vec<(Timestamp,D::K)> {
    match self.instance_map.get( &instance ) {
      None => Vec::new(),
      Some(imd) => {
        imd.instance_samples.iter()
          .filter_map( |ts| 
                if self.sample_selector( &rc, &imd, self.datasamples.get(&ts).unwrap() ) {
                  Some( (ts.clone(),instance.clone()) )   
                } else { None }
            )
          .collect()
      }
    }
  }

  // select helper
  fn sample_selector(&self, rc: &ReadCondition, imd: &InstanceMetaData, d: &SampleWithMetaData<D> ) -> bool {
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


  // read methods perform actual read or take. They must be called with key vectors
  // obtained from select_*_for_access -methods above, or their subvectors.
  pub fn read_by_keys(&mut self, keys: &[(Timestamp,D::K)] ) -> Vec<DataSample<&D>> {
    let len = keys.len();    
    let mut result = Vec::with_capacity(len);
    let mut instance_generations : HashMap<D::K,NotAliveGenerationCounts> = HashMap::new();
    if len > 0 {
      let mrsic_total = self.instance_map.get( &keys.last().unwrap().1 ).unwrap()
                          .latest_generation_available.total();
      let mrs_total = self.datasamples.iter()
                        .next_back().unwrap().1
                        .generation_counts.total();
      let mut sample_infos = VecDeque::with_capacity(len);
      // construct SampleInfos
      for (index,(ts,key)) in keys.iter().enumerate() {
        let dswm = self.datasamples.get_mut(ts).unwrap();
        let imd = self.instance_map.get(key).unwrap();
        let sample_info = SampleInfo {
          sample_state: if dswm.sample_has_been_read { SampleState::Read } else { SampleState::NotRead },
          view_state: if dswm.generation_counts.total() > imd.last_generation_accessed.total() { 
            ViewState::New } else { ViewState::NotNew },
          instance_state: imd.instance_state,
          generation_counts: dswm.generation_counts.clone(),
          sample_rank: (len - index - 1) as i32, // how many samples follow this one
          generation_rank: mrsic_total - dswm.generation_counts.total(),
          absolute_generation_rank: mrs_total - dswm.generation_counts.total(),
          source_timestamp: dswm.source_timestamp.clone(),
          publication_handle: dswm.writer_guid,
        };
        dswm.sample_has_been_read = true; // mark read
        // record instance generation accessed
        instance_generations.entry(key.clone())
          .and_modify( |old_gens| 
                if dswm.generation_counts.total() > old_gens.total() 
                  { *old_gens = dswm.generation_counts }  
              )
          .or_insert( dswm.generation_counts );
        sample_infos.push_back(sample_info);
      } 

      // mark instances viewed
      for (inst,gen) in instance_generations.iter() {
        if let Some(imd) = self.instance_map.get_mut(inst) {
          imd.last_generation_accessed = *gen;
        } else { panic!("Instance disappeared!?!!1!") }
      }
    
      // We need to do SampleInfo construction and final result construction as separate passes.
      // This is becaue SampleInfo construction needs to mark items as read and generations 
      // as viewed, i.e. needs mutable reference to data_samples.
      // Result construction (in read, not take) needs to hand out multiple references into
      // data_samples, therefore it needs immutable access, not mutable.

      // construct results
      for (ts,_key) in keys.iter() {
          let sample_info = sample_infos.pop_front().unwrap();
          let sample : &std::result::Result<D, D::K> 
              = &self.datasamples.get(ts).unwrap().sample;
          result.push( DataSample::new(sample_info, result_ok_as_ref_err_clone( sample ) ) );
      }
    } // if len > 0

    // return result
    result
  }

  pub fn take_by_keys(&mut self, keys: &[(Timestamp,D::K)] ) -> Vec<DataSample<D>> {
    let mut result = Vec::with_capacity(keys.len());
    let len = keys.len();
    let mut instance_generations : HashMap<D::K,NotAliveGenerationCounts> = HashMap::new();
    if len > 0 {
      let mrsic_total = self.instance_map.get( &keys.last().unwrap().1 ).unwrap()
                          .latest_generation_available.total();
      let mrs_total = self.datasamples.iter()
                        .next_back().unwrap().1
                        .generation_counts.total();
      // collect result
      for (index,(ts,key)) in keys.iter().enumerate() {
        let dswm = self.datasamples.remove(ts).unwrap();
        let imd = self.instance_map.get(key).unwrap();
        let sample_info = SampleInfo {
          sample_state: if dswm.sample_has_been_read { SampleState::Read } else { SampleState::NotRead },
          view_state: if dswm.generation_counts.total() > imd.last_generation_accessed.total() { 
            ViewState::New } else { ViewState::NotNew },
          instance_state: imd.instance_state,
          generation_counts: dswm.generation_counts.clone(),
          sample_rank: (len-index-1) as i32, // how many samples follow this one
          generation_rank: mrsic_total - dswm.generation_counts.total(),
          absolute_generation_rank: mrs_total - dswm.generation_counts.total(),
          source_timestamp: dswm.source_timestamp.clone(),
          publication_handle: dswm.writer_guid,
        };
        //dwsm.sample_has_been_read = true; // mark read
        // record instance generation accessed
        instance_generations.entry(key.clone())
          .and_modify( |old_gens| 
              if dswm.generation_counts.total() > old_gens.total() 
                { *old_gens = dswm.generation_counts.clone() }
            )  
          .or_insert( dswm.generation_counts.clone() );

        result.push( DataSample::new(sample_info, dswm.sample ) );
      } // for

      // mark instances viewed
      for (inst,gen) in instance_generations.iter() {
        if let Some(imd) = self.instance_map.get_mut(inst) {
          imd.last_generation_accessed = *gen;
        } else { panic!("Instance disappeared!?!!1!") }
      }
    }

    result
  }

  pub fn read_bare_by_keys (&mut self, keys: &[(Timestamp,D::K)] ) -> Vec<&D> {
    unimplemented!();
  }

  pub fn take_bare_by_keys (&mut self, keys: &[(Timestamp,D::K)] ) -> Vec<D> {
    unimplemented!();    
  }


  /*
  pub fn add_datasample(&mut self, data_sample: DataSample<D>) {
    let key: D::K = data_sample.get_key();

    if !self.distinct_keys.contains(&key) {
      debug!("Adding key with hash {:x?}", key.into_hash_key());
      self.distinct_keys.insert(key.clone());
    }

    let block = self.datasamples.get_mut(&key);

    match self.qos.history() {
      Some(history) => match history {
        History::KeepAll => match block {
          Some(prev_samples) => prev_samples.push(data_sample),
          None => {
            self.datasamples.insert(key.clone(), vec![data_sample]);
            // TODO: Check if DDS resource limits are exceeded by this insert, and
            // discard older samples as needed.
          }
        },
        History::KeepLast { depth } => match block {
          Some(prev_samples) => {
            prev_samples.push(data_sample);
            let val = prev_samples.len() - *depth as usize;
            prev_samples.drain(0..val);
          }
          None => {
            self.datasamples.insert(key.clone(), vec![data_sample]);
          }
        },
      },
      None => match block {
        // using keep last 1 as default history policy
        Some(prev_samples) => {
          prev_samples.push(data_sample);
          let val = prev_samples.len() - 1;
          prev_samples.drain(0..val);
        }
        None => {
          self.datasamples.insert(key.clone(), vec![data_sample]);
        }
      },
    }
  }
  */

  /* this seems to be for tests only?
  pub fn get_datasample(&self, key: &D::K) -> Option<&Vec<DataSample<D>>> {
    self.datasamples.get(&key)
  }
  */

  pub fn get_key_by_hash(&self, key_hash: u128) -> Option<D::K> {
    self.hash_to_key_map.get(&key_hash).map(|key| key.clone())
  }

  /*pub fn get_datasamples_mut(&mut self, key: &D::K) -> Option<&mut Vec<DataSample<D>>> {
    self.datasamples.get_mut(&key)
  }*/

  pub fn get_next_key(&self, key: &D::K) -> Option<D::K> {
    self.instance_map.range( (Excluded(key),Unbounded) ).map( |(k,_)| k.clone() ).next()
    /*
    if let Some(pos) = self.datasamples.iter().position(|(k, _)| k == key) {
      if let Some(next) = self.datasamples.iter().nth(pos + 1) {
        return Some(next.0.clone());
      };
    };
    None
    */
  }
  
  /*
  pub fn remove_datasamples(&mut self, key: &D::K) -> Option<Vec<DataSample<D>>> {
    self.datasamples.remove(&key)
  }
  */

  pub fn set_qos_policy(&mut self, qos: QosPolicies) {
    self.qos = qos
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    structure::{guid::GUID, time::Timestamp},
  };
  use crate::dds::ddsdata::DDSData;
  use crate::dds::traits::key::Keyed;
  use crate::test::random_data::*;

  #[test]
  fn dsc_empty_qos() {
    let qos = QosPolicies::qos_none();
    let mut datasample_cache = DataSampleCache::<RandomData>::new(qos);

    let timestamp = Timestamp::from(time::get_time());
    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    let org_ddsdata = DDSData::from(&data, Some(timestamp));

    let key = data.get_key().clone();
    let datasample = DataSample::new(timestamp, data.clone(), GUID::GUID_UNKNOWN);
    datasample_cache.add_datasample(datasample);
    //datasample_cache.add_datasample(datasample).unwrap();

    let samples = datasample_cache.get_datasample(&key);
    match samples {
      Some(ss) => {
        assert_eq!(ss.len(), 1);
        match &ss.get(0).unwrap().value() {
          Ok(huh) => {
            let ddssample = DDSData::from(huh, Some(timestamp));
            assert_eq!(org_ddsdata, ddssample);
          }
          _ => (),
        }
      }
      None => assert!(false),
    }
  }
}
