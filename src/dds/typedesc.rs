#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TypeDesc {
  my_name: String, // this is a rather minimal implementation
} // placeholders

impl TypeDesc {
  pub fn new(my_name: String) -> TypeDesc {
    TypeDesc { my_name }
  }
  pub fn name(&self) -> &str {
    &self.my_name
  }
}
