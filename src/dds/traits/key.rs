// This module defines traits to specifiy a key as defined in DDS specification.
// See e.g. Figure 2.3 in "2.2.1.2.2 Overall Conceptual Model"
use std::hash::Hash;
use rand::Rng;
use serde::Serialize;

pub trait Keyed
where
  Self::K: Key,
{
  type K; // key type
  fn get_key(&self) -> &Self::K;

  fn default() -> Self;
}

pub trait Key: Eq + PartialEq + PartialOrd + Ord + Hash {
  // no methods
}

#[derive(Eq, PartialEq, PartialOrd, Ord, Hash)]
// Key type to identicy data instances in builtin topics
pub struct BuiltInTopicKey {
  // IDL PSM (2.3.3, pg 138) uses array of 3x long to implement this
  value: [i32; 3],
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize)]
pub struct DefaultKey {
  value: i64,
}

impl DefaultKey {
  pub fn new(val: i64) -> DefaultKey {
    DefaultKey { value: val }
  }

  pub fn random_key() -> DefaultKey {
    let mut rng = rand::thread_rng();
    DefaultKey::new(rng.gen())
  }

  pub fn default() -> DefaultKey {
    DefaultKey::new(0)
  }
}

impl Key for DefaultKey {}
