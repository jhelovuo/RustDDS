use std::collections::HashMap;

use crate::dds::datasample::DataSample;
use crate::dds::traits::key::DefaultKey;
use crate::dds::ddsdata::DDSData;

pub struct DataSampleCache {
  datasamples: HashMap<DefaultKey, DataSample<DDSData>>,
}

impl DataSampleCache {
  pub fn new() -> DataSampleCache {
    DataSampleCache {
      datasamples: HashMap::new(),
    }
  }
}
