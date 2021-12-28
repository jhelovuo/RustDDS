use std::hash::Hash;

use serde::{Deserialize, Serialize};

#[allow(unused_imports)] // since this is testing code only
use crate::{
  dds::traits::key::{Key, Keyed},
  serialization::cdr_serializer::to_bytes,
};

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Clone, Hash)]
pub struct RandomKey {
  val: i64,
}

impl RandomKey {
  #[allow(dead_code)] // just testing
  pub fn new(val: i64) -> RandomKey {
    RandomKey { val }
  }
}

//impl Key for RandomKey {
//}

#[derive(Serialize, Debug, Clone, PartialEq, Deserialize)]
pub struct RandomData {
  pub a: i64,
  pub b: String,
}

impl Keyed for RandomData {
  type K = i64;
  fn key(&self) -> i64 {
    self.a
  }
}
