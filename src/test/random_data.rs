use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use serde::Serialize;

use crate::dds::traits::{
  key::{Key, Keyed},
  //datasample_trait::DataSampleTrait,
};

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Clone, Hash)]
pub struct RandomKey {
  val: i64,
}

impl RandomKey {
  pub fn new(val: i64) -> RandomKey {
    RandomKey { val }
  }
}

//impl Key for RandomKey {
//}

#[derive(Serialize, Debug, Clone)]
pub struct RandomData {
  pub a: i64,
  pub b: String,
}

impl Keyed for RandomData {
  type K = i64;
  fn get_key(&self) -> i64 {
    self.a
  }
}
