// This module defines traits to specifiy a key as defined in DDS specification.
// See e.g. Figure 2.3 in "2.2.1.2.2 Overall Conceptual Model"
use std::hash::Hash;
use rand::Rng;

pub trait Keyed: Sync + Send {
  fn get_key(&self) -> Box<dyn Key>;
}

// pub trait Key: Eq + PartialEq + PartialOrd + Ord + Hash + Clone {
//   // no methods
// }
pub trait Key: Sync + Send {
  fn get_hash(&self) -> u64;
  fn box_clone(&self) -> Box<dyn Key>;
}

impl Clone for Box<dyn Key> {
  fn clone(&self) -> Self {
    (*self).box_clone()
  }
}

#[derive(Eq, PartialEq, PartialOrd, Ord, Hash)]
// Key type to identicy data instances in builtin topics
pub struct BuiltInTopicKey {
  // IDL PSM (2.3.3, pg 138) uses array of 3x long to implement this
  value: [i32; 3],
}

impl BuiltInTopicKey {
  pub fn get_random_key() -> BuiltInTopicKey {
    let mut rng = rand::thread_rng();
    BuiltInTopicKey {
      value: [rng.gen(), rng.gen(), rng.gen()],
    }
  }

  pub fn default() -> BuiltInTopicKey {
    BuiltInTopicKey { value: [0, 0, 0] }
  }
}
