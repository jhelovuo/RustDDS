use crate::dds::traits::key::Key;
use crate::dds::datasample::DataSample;
use crate::dds::traits::datasample_trait::DataSampleTrait;
use crate::dds::values::result::{Result, Error};
use crate::dds::qos::QosPolicies;
use crate::dds::qos::policy::History;
use std::collections::HashMap;

pub struct DataSampleCache {
  qos: QosPolicies,
  datasamples: HashMap<u64, Vec<DataSample>>,
}

impl DataSampleCache {
  pub fn new(qos: QosPolicies) -> DataSampleCache {
    DataSampleCache {
      qos,
      datasamples: HashMap::new(),
    }
  }

  pub fn add_datasample<D: DataSampleTrait>(
    &mut self,
    data_sample: DataSample,
  ) -> Result<Box<dyn Key>> {
    let key = match &data_sample.value {
      Ok(v) => v.get_key().box_clone(),
      _ => return Err(Error::OutOfResources),
    };

    let block = self.datasamples.get_mut(&key.get_hash());

    match self.qos.history() {
      Some(history) => match history {
        History::KeepAll => match block {
          Some(prev_samples) => prev_samples.push(data_sample),
          None => {
            self.datasamples.insert(key.get_hash(), vec![data_sample]);
          }
        },
        History::KeepLast { depth } => match block {
          Some(prev_samples) => {
            prev_samples.push(data_sample);
            let val = prev_samples.len() - *depth as usize;
            prev_samples.drain(0..val);
          }
          None => {
            self.datasamples.insert(key.get_hash(), vec![data_sample]);
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
          self.datasamples.insert(key.get_hash(), vec![data_sample]);
        }
      },
    }
    Ok(key)
  }

  pub fn get_datasample(&self, key: Box<dyn Key>) -> Option<&Vec<DataSample>> {
    let values = self.datasamples.get(&key.get_hash());
    values
  }

  pub fn set_qos_policy(&mut self, qos: QosPolicies) {
    self.qos = qos
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::structure::time::Timestamp;
  use serde::Serialize;
  use crate::dds::traits::datasample_trait::DataSampleTrait;
  use std::hash::{Hash, Hasher};
  use std::collections::hash_map::DefaultHasher;
  use crate::dds::ddsdata::DDSData;
  use crate::dds::traits::key::Keyed;

  struct RandomKey {
    val: i64,
  }

  impl RandomKey {
    pub fn new(val: i64) -> RandomKey {
      RandomKey { val }
    }
  }

  impl Key for RandomKey {
    fn get_hash(&self) -> u64 {
      let mut hasher = DefaultHasher::new();
      self.val.hash(&mut hasher);
      hasher.finish()
    }

    fn box_clone(&self) -> Box<dyn Key> {
      let n = RandomKey::new(self.val);
      Box::new(n)
    }
  }

  #[derive(Serialize, Debug, Clone)]
  struct RandomData {
    a: i64,
    b: String,
  }

  impl Keyed for RandomData {
    fn get_key(&self) -> Box<dyn Key> {
      let key = RandomKey::new(self.a);
      Box::new(key)
    }
  }

  impl DataSampleTrait for RandomData {
    fn box_clone(&self) -> Box<dyn DataSampleTrait> {
      Box::new(RandomData {
        a: self.a.clone(),
        b: self.b.clone(),
      })
    }
  }

  #[test]
  fn dsc_empty_qos() {
    let qos = QosPolicies::qos_none();
    let mut datasample_cache = DataSampleCache::new(qos);

    let timestamp = Timestamp::from(time::get_time());
    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    let org_ddsdata = DDSData::from(&data, Some(timestamp));

    let key = data.get_key().clone();
    let datasample = DataSample::new(timestamp, data.clone());
    datasample_cache
      .add_datasample::<RandomData>(datasample.clone())
      .unwrap();
    datasample_cache
      .add_datasample::<RandomData>(datasample)
      .unwrap();

    let samples = datasample_cache.get_datasample(key);
    match samples {
      Some(ss) => {
        assert_eq!(ss.len(), 1);
        match &ss.get(0).unwrap().get_value_with_type::<RandomData>() {
          Some(huh) => {
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
