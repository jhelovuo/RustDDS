use crate::dds::traits::key::{Keyed, Key};
// use crate::dds::datasample::DataSample;

use crate::dds::values::result::{Result, Error};
use std::any::Any;

pub struct DataSampleCache {
  datasamples: Vec<Box<dyn Keyed + Send + Sync>>,
}

impl DataSampleCache {
  pub fn new() -> DataSampleCache {
    DataSampleCache {
      datasamples: Vec::new(),
    }
  }

  pub fn add_data_sample<D: Keyed + Send + Sync + 'static>(
    &mut self,
    data_sample: D,
  ) -> Result<Box<dyn Key>> {
    // pretty slow but should avoids problems with generic type
    for ds in self.datasamples.iter() {
      let typeid = ds.type_id();
      if typeid != data_sample.type_id() {
        continue;
      }

      if ds.get_key().get_hash() == data_sample.get_key().get_hash() {
        return Err(Error::OutOfResources);
      }
    }

    let key = data_sample.get_key().box_clone();
    self.datasamples.push(Box::new(data_sample));
    Ok(key)

    // let key = match data_sample.value {
    //   Ok(ad) => (*ad).get_key().clone(),
    //   Err(e) => e,
    // };
    // match self.datasamples.insert(key, data_sample) {
    //   Some(_) => {
    //     println!("warning: for some reason key existed");
    //     Ok(key)
    //   }
    //   None => Ok(key),
    // }
  }
}
