use std::collections::HashMap;

use crate::dds::datasample::DataSample;
use crate::dds::traits::key::DefaultKey;
use crate::dds::ddsdata::DDSData;

use crate::dds::values::result::Result;

pub struct DataSampleCache {
  datasamples: HashMap<DefaultKey, DataSample<DDSData>>,
}

impl DataSampleCache {
  pub fn new() -> DataSampleCache {
    DataSampleCache {
      datasamples: HashMap::new(),
    }
  }

  pub fn add_data_sample(&mut self, data_sample: DataSample<DDSData>) -> Result<DefaultKey> {
    let key = self.get_free_key();
    match self.datasamples.insert(key.clone(), data_sample) {
      Some(_) => {
        println!("warning: for some reason key existed");
        Ok(key)
      }
      None => Ok(key),
    }
  }

  fn get_free_key(&self) -> DefaultKey {
    let mut key = DefaultKey::random_key();
    while self.datasamples.contains_key(&key) {
      key = DefaultKey::random_key();
    }
    key
  }
}
