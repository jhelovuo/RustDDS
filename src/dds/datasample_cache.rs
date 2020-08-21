use crate::dds::traits::key::{Key, Keyed};
use crate::dds::datasample::DataSample;
use crate::dds::values::result::Result;
use crate::dds::qos::QosPolicies;
use crate::dds::qos::policy::History;

use std::collections::BTreeMap;

pub struct DataSampleCache<D: Keyed> {
  qos: QosPolicies,
  pub datasamples: BTreeMap<D::K, Vec<DataSample<D>>>,
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
    }
  }

  pub fn add_datasample(&mut self, data_sample: DataSample<D>) {
    let key: D::K = data_sample.get_key();
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

  pub fn get_datasample(&self, key: &D::K) -> Option<&Vec<DataSample<D>>> {
    self.datasamples.get(&key)
  }

  pub fn get_datasamples_mut(&mut self, key: &D::K) -> Option<&mut Vec<DataSample<D>>> {
    self.datasamples.get_mut(&key)
  }

  pub fn get_next_key(&self, key: &D::K) -> D::K {
    let pos = self.datasamples.iter().position(|(k, _)| k == key);
    self
      .datasamples
      .iter()
      .nth(pos.unwrap() + 1)
      .unwrap()
      .0
      .clone()
  }

  pub fn remove_datasamples(&mut self, key: &D::K) {
    self.datasamples.remove(&key).unwrap();
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
    let datasample = DataSample::new(timestamp, data.clone());
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
