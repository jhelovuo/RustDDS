use itertools::Itertools;
use log::debug;

use crate::{
  dds::traits::key::{Key, Keyed},
  structure::guid::GUID,
};
use crate::dds::datasample::DataSample;
//use crate::dds::values::result::Result;
use crate::dds::qos::QosPolicies;
use crate::dds::qos::policy::History;

use std::collections::{BTreeMap, HashSet};

use super::datasample::SampleInfo;

pub struct DataSampleCache<D: Keyed> {
  qos: QosPolicies,
  pub datasamples: BTreeMap<D::K, Vec<DataSample<D>>>,
  distinct_keys: HashSet<D::K>,
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
      distinct_keys: HashSet::new(),
    }
  }

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
            let distinct_writers: Vec<GUID> = prev_samples
              .iter()
              .map(|p| p.sample_info.publication_handle)
              .unique()
              .collect();
            let mut fsamples = Vec::new();
            for g in distinct_writers.into_iter() {
              let mut tmp: Vec<SampleInfo> = prev_samples
                .into_iter()
                .filter(|p| p.sample_info.publication_handle == g)
                .map(|p| p.sample_info.clone())
                .take((*depth) as usize)
                .collect();
              fsamples.append(&mut tmp);
            }
            prev_samples.retain(|p| fsamples.iter().find(|&c| *c == p.sample_info).is_some());
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
          let distinct_writers: Vec<GUID> = prev_samples
            .iter()
            .map(|p| p.sample_info.publication_handle)
            .unique()
            .collect();
          let mut fsamples = Vec::new();
          for g in distinct_writers.into_iter() {
            let mut tmp: Vec<SampleInfo> = prev_samples
              .into_iter()
              .filter(|p| p.sample_info.publication_handle == g)
              .map(|p| p.sample_info.clone())
              .take(1)
              .collect();
            fsamples.append(&mut tmp);
          }
          prev_samples.retain(|p| fsamples.iter().find(|&c| *c == p.sample_info).is_some());
        }
        None => {
          self.datasamples.insert(key.clone(), vec![data_sample]);
        }
      },
    }
  }

  pub fn get_datasample(&self, key: &D::K) -> Option<&Vec<DataSample<D>>> {
    self.datasamples.get(&key)
  }

  pub fn get_key(&self, key_hash: u128) -> Option<D::K> {
    self
      .distinct_keys
      .iter()
      .find(|key| key.into_hash_key() == key_hash)
      .map(|key| key.clone())
  }

  pub fn get_datasamples_mut(&mut self, key: &D::K) -> Option<&mut Vec<DataSample<D>>> {
    self.datasamples.get_mut(&key)
  }

  pub fn get_next_key(&self, key: &D::K) -> Option<D::K> {
    if let Some(pos) = self.datasamples.iter().position(|(k, _)| k == key) {
      if let Some(next) = self.datasamples.iter().nth(pos + 1) {
        return Some(next.0.clone());
      };
    };
    None
  }

  pub fn remove_datasamples(&mut self, key: &D::K) -> Option<Vec<DataSample<D>>> {
    self.datasamples.remove(&key)
  }

  pub fn set_qos_policy(&mut self, qos: QosPolicies) {
    self.qos = qos
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::structure::time::Timestamp;
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
        match &ss.get(0).unwrap().value {
          Ok(huh) => {
            let ddssample = DDSData::from(&*huh, Some(timestamp));
            assert_eq!(org_ddsdata, ddssample);
          }
          _ => (),
        }
      }
      None => assert!(false),
    }
  }
}
