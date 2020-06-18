macro_rules! checked_impl {
  ($trait_name:ident, $method:ident, $t:ty) => {
    use num_traits::$trait_name;

    impl $trait_name for $t {
      #[inline]
      fn $method(&self, v: &$t) -> Option<$t> {
        (self.0).$method(v.0).map(|value| <$t>::from(value))
      }
    }
  };
}
