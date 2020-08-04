use serde::Deserialize;

use crate::structure::instance_handle::InstanceHandle;

use crate::structure::entity::{Entity, EntityAttributes};

use crate::dds::values::result::*;
use crate::dds::traits::key::*;
use crate::dds::qos::*;
use crate::dds::datasample::*;
use crate::dds::datasample_cache::DataSampleCache;
use crate::structure::guid::{GUID};

pub struct DataReader {
  //my_subscriber: &'s Subscriber<'s>,
  qos_policy: QosPolicies,
  entity_attributes: EntityAttributes,
  datasample_cache: DataSampleCache,
  // TODO: rest of fields
}

// TODO: rewrite DataSample so it can use current Keyed version (and send back datasamples instead of current data)

impl<'s> DataReader {
  pub fn new(qos: QosPolicies) -> Self {
    Self {
      qos_policy: qos,
      entity_attributes: EntityAttributes::new(GUID::new()), // todo
      datasample_cache: DataSampleCache::new(),
    }
  }

  pub fn add_datasample<D>(&self, _datasample: D) -> Result<()>
  where
    D: Deserialize<'s> + Keyed,
  {
    Ok(())
  }

  pub fn read<D>(
    &self,
    _max_samples: i32,
    _sample_state: SampleState,
    _view_state: ViewState,
    _instance_state: InstanceState,
  ) -> Result<Vec<D>>
  where
    D: Deserialize<'s> + Keyed,
  {
    unimplemented!();
    // Go through the historycache list and return all relevant in a vec.
  }

  pub fn take<D>(
    &self,
    _max_samples: i32,
    _sample_state: SampleState,
    _view_state: ViewState,
    _instance_state: InstanceState,
  ) -> Result<Vec<D>>
  where
    D: Deserialize<'s> + Keyed,
  {
    unimplemented!()
  }

  pub fn read_next<D>(&self) -> Result<Vec<D>>
  where
    D: Deserialize<'s> + Keyed,
  {
    todo!()
  }

  pub fn read_instance<D>(&self, _instance_handle: InstanceHandle) -> Result<Vec<D>>
  where
    D: Deserialize<'s> + Keyed,
  {
    todo!()
  }
} // impl

impl Entity for DataReader {
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
  }
}
