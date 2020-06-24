// This module defines traits to specifiy a key as defined in DDS specification.
// See e.g. Figure 2.3 in "2.2.1.2.2 Overall Conceptual Model"
use std::hash::Hash;

pub trait Keyed
where
  Self::K: Key,
{
  type K; // key type
  fn get_key(self) -> Self::K;
}

pub trait Key: Eq + PartialEq + PartialOrd + Ord + Hash {
  // no methods
}

// Key type to identicy data instances in builtin topics
pub struct BuiltInTopicKey {
  // IDL PSM (2.3.3, pg 138) uses array of 3x long to implement this
  value: [i32; 3],
}
