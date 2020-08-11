use crate::dds::traits::key::Keyed;
use erased_serde::{serialize_trait_object, Serialize};
use std::fmt::Debug;
//use downcast_rs::{impl_downcast, DowncastSync};
/*
pub trait DataSampleTrait: Keyed + Serialize + DowncastSync {
  fn box_clone(&self) -> Box<dyn DataSampleTrait>;
}

impl Debug for dyn DataSampleTrait {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Some debug info for DataSampleTrait")
  }
}

impl PartialEq for Box<dyn DataSampleTrait> {
  fn eq(&self, other: &Self) -> bool {
    (*self).get_key().get_hash() == other.get_key().get_hash()
  }
}

impl Clone for Box<dyn DataSampleTrait> {
  fn clone(&self) -> Box<dyn DataSampleTrait> {
    (*self).box_clone()
  }
}

serialize_trait_object!(DataSampleTrait);
impl_downcast!(sync DataSampleTrait);
*/