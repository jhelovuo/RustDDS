use crate::{
  dds::qos::QosPolicies,
  security::{access_control::*, *},
  security_error,
};
use super::{
  domain_governance_document::TopicRule,
  types::{BuiltinPluginEndpointSecurityAttributes, Entity},
  AccessControlBuiltin,
};

impl AccessControlBuiltin {
  fn get_endpoint_security_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: &str,
  ) -> SecurityResult<EndpointSecurityAttributes> {
    self
      .get_domain_rule(&permissions_handle)
      .and_then(|domain_rule| {
        domain_rule.find_topic_rule(topic_name).ok_or_else(|| {
          security_error!("Could not find a topic rule for the topic_name {topic_name}")
        })
      })
      .map(
        |TopicRule {
           enable_discovery_protection,
           enable_liveliness_protection,
           enable_read_access_control,
           enable_write_access_control,
           metadata_protection_kind,
           data_protection_kind,
           ..
         }| {
          let (
            is_submessage_protected,
            is_submessage_encrypted,
            is_submessage_origin_authenticated,
          ) = metadata_protection_kind.to_security_attributes_format();
          let (is_payload_protected, is_payload_encrypted, is_key_protected) =
            data_protection_kind.to_security_attributes_format();
          EndpointSecurityAttributes {
            topic_security_attributes: TopicSecurityAttributes {
              is_read_protected: *enable_read_access_control,
              is_write_protected: *enable_write_access_control,
              is_discovery_protected: *enable_discovery_protection,
              is_liveliness_protected: *enable_liveliness_protection,
            },
            is_submessage_protected,
            is_payload_protected,
            is_key_protected,
            plugin_endpoint_attributes: BuiltinPluginEndpointSecurityAttributes {
              is_submessage_encrypted,
              is_submessage_origin_authenticated,
              is_payload_encrypted,
            }
            .into(),
            ac_endpoint_properties: Vec::new(),
          }
        },
      )
  }
}

impl LocalEntityAccessControl for AccessControlBuiltin {
  fn check_create_datawriter(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    _qos: &QosPolicies,
  ) -> SecurityResult<()> {
    let partitions = &[]; // Partitions currently unsupported. TODO: get from PartitionQosPolicy
    let data_tags = &[]; // Data tagging currently unsupported. TODO: get from DataTagQosPolicy
    self.check_entity(
      permissions_handle,
      domain_id,
      &topic_name,
      partitions,
      data_tags,
      &Entity::Datawriter,
    )
  }

  fn check_create_datareader(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    _qos: &QosPolicies,
  ) -> SecurityResult<()> {
    let partitions = &[]; // Partitions currently unsupported. TODO: get from PartitionQosPolicy
    let data_tags = &[]; // Data tagging currently unsupported. TODO: get from DataTagQosPolicy
    self.check_entity(
      permissions_handle,
      domain_id,
      &topic_name,
      partitions,
      data_tags,
      &Entity::Datareader,
    )
  }

  fn check_create_topic(
    &self,
    permissions_handle: PermissionsHandle,
    domain_id: u16,
    topic_name: String,
    _qos: &QosPolicies,
  ) -> SecurityResult<()> {
    let partitions = &[]; // Partitions currently unsupported. TODO: get from PartitionQosPolicy
    let data_tags = &[]; // Data tagging currently unsupported. TODO: get from DataTagQosPolicy
    self.check_entity(
      permissions_handle,
      domain_id,
      &topic_name,
      partitions,
      data_tags,
      &Entity::Topic,
    )
  }

  fn check_local_datawriter_register_instance(
    &self,
    _permissions_handle: PermissionsHandle,
    _writer_todo: (),
    _key_todo: (),
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }

  fn check_local_datawriter_dispose_instance(
    &self,
    _permissions_handle: PermissionsHandle,
    _writer_todo: (),
    _key_todo: (),
  ) -> SecurityResult<()> {
    // According to 9.4.3 this actually just returns OK, probably reserved for
    // custom plugins
    Ok(())
  }

  // Currently only mocked, but ready after removing the last line
  fn get_topic_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: String,
  ) -> SecurityResult<TopicSecurityAttributes> {
    self
      .get_endpoint_security_attributes(permissions_handle, &topic_name)
      .map(
        |EndpointSecurityAttributes {
           topic_security_attributes,
           ..
         }| topic_security_attributes,
      )
      // TODO remove after testing
      .or(Ok(TopicSecurityAttributes::empty()))
  }

  // Currently only mocked, but ready after removing the last line
  fn get_datawriter_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: String,
  ) -> SecurityResult<EndpointSecurityAttributes> {
    self
      .get_endpoint_security_attributes(permissions_handle, &topic_name)
      // TODO remove after testing
      .or(Ok(EndpointSecurityAttributes::empty()))
  }

  // Currently only mocked, but ready after removing the last line
  fn get_datareader_sec_attributes(
    &self,
    permissions_handle: PermissionsHandle,
    topic_name: String,
  ) -> SecurityResult<EndpointSecurityAttributes> {
    self
      .get_endpoint_security_attributes(permissions_handle, &topic_name)
      // TODO remove after testing
      .or(Ok(EndpointSecurityAttributes::empty()))
  }
}
