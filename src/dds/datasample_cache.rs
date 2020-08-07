use crate::dds::traits::key::Key;
use crate::dds::datasample::DataSample;
use crate::dds::traits::datasample_trait::DataSampleTrait;
use crate::dds::values::result::Result;
use crate::dds::qos::QosPolicies;
use crate::dds::qos::policy::History;
use crate::structure::instance_handle::InstanceHandle;
use std::collections::HashMap;

pub struct DataSampleCache<D> {
  qos: QosPolicies,
  datasamples: HashMap<u64, Vec<DataSample>>,
  phantom: std::marker::PhantomData<D>, // this is a placeholder to prevent errors until D is actually used here.
}

impl<D> DataSampleCache<D> 
where D: DataSampleTrait
{
  pub fn new(qos: QosPolicies) -> DataSampleCache<D> {
    DataSampleCache {
      qos,
      datasamples: HashMap::new(),
      phantom: std::marker::PhantomData::<D>, // this is a placeholder to prevent errors until D is actually used here.
    }
  }

  pub fn add_datasample(
    &mut self,
    data_sample: DataSample,
  ) -> Result<Box<dyn Key>> {
    let key = match &data_sample.value {
      Ok(v) => v.get_key().box_clone(),
      Err(key) => key.box_clone(),
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

  pub fn get_datasample(&self, key: &Box<dyn Key>) -> Option<&Vec<DataSample>> {
    let values = self.datasamples.get(&key.get_hash());
    values
  }

  pub fn remove_datasamples(&mut self, key: &Box<dyn Key>) {
    self.datasamples.remove(&key.get_hash()).unwrap();
  }

  pub fn set_qos_policy(&mut self, qos: QosPolicies) {
    self.qos = qos
  }

  pub fn generate_free_instance_handle(&self) -> InstanceHandle {
    let mut instance_handle = InstanceHandle::generate_random_key();

    loop {
      let mut has_key = false;
      for (_, cc) in &self.datasamples {
        for ds in cc {
          if ds.instance_handle == instance_handle {
            has_key = true;
          }
        }
      }

      if !has_key {
        return instance_handle;
      }

      instance_handle = InstanceHandle::generate_random_key();
    }
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
    let mut datasample_cache = DataSampleCache::new(qos);

    let timestamp = Timestamp::from(time::get_time());
    let data = RandomData {
      a: 4,
      b: "Fobar".to_string(),
    };

    let instance_handle = datasample_cache.generate_free_instance_handle();
    let org_ddsdata = DDSData::from(instance_handle.clone(), &data, Some(timestamp));

    let key = data.get_key().clone();
    let datasample = DataSample::new(timestamp, instance_handle.clone(), data.clone());
    datasample_cache
      .add_datasample::<RandomData>(datasample.clone())
      .unwrap();
    datasample_cache
      .add_datasample::<RandomData>(datasample)
      .unwrap();

    let samples = datasample_cache.get_datasample(&key);
    match samples {
      Some(ss) => {
        assert_eq!(ss.len(), 1);
        match &ss.get(0).unwrap().get_value_with_type::<RandomData>() {
          Some(huh) => {
            let ddssample = DDSData::from(instance_handle.clone(), huh, Some(timestamp));
            assert_eq!(org_ddsdata, ddssample);
          }
          _ => (),
        }
      }
      None => assert!(false),
    }
  }
}
