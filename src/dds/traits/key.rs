// This module defines traits to specifiy a key as defined in DDS specification.
// See e.g. Figure 2.3 in "2.2.1.2.2 Overall Conceptual Model"
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use rand::Rng;
use serde::{Serialize, Deserialize, de::DeserializeOwned};

// A payload data object may be "Keyed": It allows a Key to be extracted from it.
// The key is used to distinguish between different Instances of the data.
// A "Keyed" data has on associated type "K", which is the actual key type. K must
// implement "Key".
pub trait Keyed {
  //type K: Key;  // This does not work yet is stable Rust, 2020-08-11
  // Instead, where D:Keyed we do anything with D::K, we must specify bound:
  // where <D as Keyed>::K : Key,
  type K;

  fn get_key(&self) -> Self::K;
  fn get_hash(&self) -> u64
  where
    Self::K: Key,
  {
    let mut hasher = DefaultHasher::new();
    self.get_key().hash(&mut hasher);
    hasher.finish()
  }
}

pub trait Key: Eq + PartialEq + PartialOrd + Ord + Hash + Clone + Serialize + DeserializeOwned {
  // no methods required

  // provides one method for convenience
  fn get_hash(&self) -> u64 {
    let mut hasher = DefaultHasher::new();
    self.hash(&mut hasher);
    hasher.finish()
  }
}

impl Key for () {
  // nothing
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
