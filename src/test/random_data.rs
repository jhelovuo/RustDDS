use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use serde::Serialize;

use crate::dds::traits::{
  key::{Key, Keyed},
  datasample_trait::DataSampleTrait,
};

pub struct RandomKey {
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
pub struct RandomData {
  pub a: i64,
  pub b: String,
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
