use crate::{
  security::{Property, SecurityError, SecurityResult},
  security_error, QosPolicies,
};

pub(super) fn get_property(properties: &[Property], property_name: &str) -> SecurityResult<String> {
  properties
    .iter()
    .find(|Property { name, .. }| name.eq(property_name))
    .map(|Property { value, .. }| value.clone())
    .ok_or_else(|| security_error!("Could not find a property of the name {}.", property_name))
}

impl QosPolicies {
  pub(super) fn get_property(&self, property_name: &str) -> SecurityResult<String> {
    self
      .property
      .as_ref()
      .ok_or_else(|| security_error!("The QosPolicies did not have any properties."))
      .and_then(|properties_or_binary_properties| {
        get_property(&properties_or_binary_properties.value, property_name)
      })
  }
}
