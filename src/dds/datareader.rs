use serde::Deserialize;

use crate::structure::instance_handle::InstanceHandle;

use crate::structure::entity::{Entity, EntityAttributes};

use crate::dds::values::result::*;
use crate::dds::traits::key::*;
use crate::dds::qos::*;
use crate::dds::datasample::*;
use crate::dds::datasample_cache::DataSampleCache;
use crate::dds::traits::datasample_trait::DataSampleTrait;
use crate::dds::pubsub::*;
use crate::structure::guid::{GUID};

pub struct DataReader<'s,D> 
{
  my_subscriber: &'s Subscriber,
  qos_policy: QosPolicies,
  entity_attributes: EntityAttributes,
  datasample_cache: DataSampleCache<D>,
  // TODO: rest of fields
}

// TODO: rewrite DataSample so it can use current Keyed version (and send back datasamples instead of current data)

impl<'d,'s,D> DataReader<'s,D> 
  where
    D: Deserialize<'d> + Keyed + DataSampleTrait,
{
  pub fn new(my_subscriber: &'s Subscriber, qos: QosPolicies) -> Self {
    Self {
      my_subscriber,
      qos_policy: qos.clone(),
      entity_attributes: EntityAttributes::new(GUID::new()), // todo
      datasample_cache: DataSampleCache::new(qos),
    }
  }

  pub fn add_datasample(&self, _datasample: D) -> Result<()>
  {
    Ok(())
  }

  pub fn read(
    &self,
    _max_samples: i32,
    _sample_state: SampleState,
    _view_state: ViewState,
    _instance_state: InstanceState,
  ) -> Result<Vec<D>>
  {
    unimplemented!();
    // Go through the historycache list and return all relevant in a vec.
  }

  pub fn take(
    &self,
    _max_samples: i32,
    _sample_state: SampleState,
    _view_state: ViewState,
    _instance_state: InstanceState,
  ) -> Result<Vec<D>>
  {
    unimplemented!()
  }

  pub fn read_next(&self) -> Result<Vec<D>>
  {
    todo!()
  }

  pub fn read_instance(&self, _instance_handle: InstanceHandle) -> Result<Vec<D>>
  {
    todo!()
  }
} // impl

impl<'a,D> Entity for DataReader<'a,D> 
  where
    D: Deserialize<'a> + Keyed + DataSampleTrait,
{
  fn as_entity(&self) -> &EntityAttributes {
    &self.entity_attributes
  }
}
