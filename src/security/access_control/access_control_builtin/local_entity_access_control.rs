use crate::{
  dds::qos::QosPolicies,
  security::{access_control::*, *},
};
use super::AccessControlBuiltin;

impl LocalEntityAccessControl for AccessControlBuiltin {
  // Currently only mocked
  fn check_create_datawriter(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    qos: &QosPolicies,
  ) -> SecurityResult<()> {
    // TODO: actual implementation

    Ok(())
  }

  // Currently only mocked
  fn check_create_datareader(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    qos: &QosPolicies,
  ) -> SecurityResult<()> {
    // TODO: actual implementation

    Ok(())
  }

  // Currently only mocked
  fn check_create_topic(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    qos: &QosPolicies,
  ) -> SecurityResult<()> {
    // TODO: actual implementation

    Ok(())
  }

  fn check_local_datawriter_register_instance(
    &self,
    permissions_handle: PermissionsHandle,
    writer_todo: (),
    key_todo: (),
  ) -> SecurityResult<()> {
    todo!();
  }

  fn check_local_datawriter_dispose_instance(
    &self,
    permissions_handle: PermissionsHandle,
    writer_todo: (),
    key_todo: (),
  ) -> SecurityResult<()> {
    todo!();
  }

  // Currently only mocked
  fn get_topic_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: String,
  ) -> SecurityResult<TopicSecurityAttributes> {
    // TODO: actual implementation

    Ok(TopicSecurityAttributes::empty())
  }

  // Currently only mocked
  fn get_datawriter_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: String,
  ) -> SecurityResult<EndpointSecurityAttributes> {
    // TODO: actual implementation

    Ok(EndpointSecurityAttributes::empty())
  }

  // Currently only mocked
  fn get_datareader_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: String,
  ) -> SecurityResult<EndpointSecurityAttributes> {
    // TODO: actual implementation

    Ok(EndpointSecurityAttributes::empty())
  }
}
