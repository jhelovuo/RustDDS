// This module defines traits to specifiy a key as defined in DDS specification.
// See e.g. Figure 2.3 in "2.2.1.2.2 Overall Conceptual Model"
use std::hash::Hash;

trait Keyed<T>
where
  Self::K: Key,
{
  type K; // key type
  fn get_key(self) -> Self::K;
}

trait Key: Eq + PartialEq + PartialOrd + Ord + Hash {
  // no methods
}
